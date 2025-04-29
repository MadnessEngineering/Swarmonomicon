#!/usr/bin/env python3
"""
WyrmWatch Server - Python equivalent of mcp_todo_server

A mad tinker's monitoring system for tasks and tiny wyrms.
Inspired by the Black Ocean series by J.S. Morin.
"""
import asyncio
import json
import os
import signal
import time
import logging
from datetime import datetime
from dataclasses import dataclass, field
from typing import Dict, Optional, List, Any, Union
import threading
from threading import Lock

# For MQTT communication
import paho.mqtt.client as mqtt

# For MongoDB integration
import pymongo
from pymongo import MongoClient
from bson import ObjectId

# For AI integration (optional)
try:
    import openai
    HAS_OPENAI = True
except ImportError:
    HAS_OPENAI = False
    print("OpenAI not available, will use simple enhancement")

# Constants
MAX_CONCURRENT_TASKS = 5
MAX_CONCURRENT_AI = 2
METRICS_REPORTING_INTERVAL = 30

# Configure logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("wyrmwatch")

# Task Priority Enum (using simple strings for Python)
class TaskPriority:
    CRITICAL = "critical"
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"

# Task Status Enum
class TaskStatus:
    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    FAILED = "failed"

class TaskMetrics:
    """Tracks task metrics like a wyrm's vital signs."""
    
    def __init__(self):
        self.tasks_received = 0
        self.tasks_processed = 0
        self.tasks_failed = 0
        self.start_time = time.time()
        self._lock = Lock()
    
    def increment_received(self):
        with self._lock:
            self.tasks_received += 1
            return self.tasks_received
    
    def increment_processed(self):
        with self._lock:
            self.tasks_processed += 1
    
    def increment_failed(self):
        with self._lock:
            self.tasks_failed += 1
    
    def as_json(self):
        """Return metrics in JSON format."""
        with self._lock:
            now = time.time()
            uptime = now - self.start_time
            return {
                "tasks_received": self.tasks_received,
                "tasks_processed": self.tasks_processed,
                "tasks_failed": self.tasks_failed,
                "uptime_seconds": int(uptime),
                "timestamp": datetime.now().isoformat()
            }

class TodoTool:
    """Handles todo operations with MongoDB."""
    
    def __init__(self, mongo_uri=None):
        """Initialize the TodoTool with MongoDB connection."""
        if mongo_uri is None:
            mongo_uri = os.environ.get("MONGODB_URI", "mongodb://localhost:27017/")
        
        self.client = MongoClient(mongo_uri)
        self.db = self.client.wyrm_todos
        self.collection = self.db.todos
        
        # Create indexes
        self.collection.create_index([("status", pymongo.ASCENDING)])
        self.collection.create_index([("priority", pymongo.ASCENDING)])
        logger.info("TodoTool initialized with MongoDB connection")
    
    async def execute(self, params: Dict[str, str]):
        """Execute a todo command."""
        command = params.get("command")
        
        if command == "add":
            return await self.add_todo(
                description=params.get("description", ""),
                context=params.get("context"),
                target_agent=params.get("target_agent", "user"),
                project=params.get("project", "wyrmwatch")
            )
        
        # Add other commands as needed (list, complete, delete, etc.)
        logger.error(f"Unknown command: {command}")
        return {"status": "error", "message": f"Unknown command: {command}"}
    
    async def add_todo(self, description: str, context: Optional[str] = None, 
                      target_agent: str = "user", project: Optional[str] = None):
        """Add a new todo with AI enhancement."""
        logger.debug(f"Adding todo - Description: {description}, Context: {context}, Target Agent: {target_agent}, Project: {project}")
        
        # Enhance the description with AI
        try:
            enhanced_description, priority, project_name = await enhance_todo_description(description)
        except Exception as e:
            logger.error(f"AI enhancement failed: {e}")
            enhanced_description = description
            priority = TaskPriority.MEDIUM
            project_name = project or "wyrmwatch"
        
        # Create todo document
        todo = {
            "description": description,
            "enhanced_description": enhanced_description,
            "status": TaskStatus.PENDING,
            "priority": priority,
            "created_at": datetime.now(),
            "updated_at": datetime.now(),
            "target_agent": target_agent,
            "context": context,
            "project": project_name,
        }
        
        # Insert into database
        logger.debug("Attempting to insert todo into database")
        result = self.collection.insert_one(todo)
        
        if result.inserted_id:
            logger.info(f"Successfully inserted todo into database: {enhanced_description}")
            return {
                "status": "success", 
                "message": f"Added new todo: {description}", 
                "timestamp": datetime.now().isoformat()
            }
        else:
            logger.error("Failed to insert todo into database")
            return {"status": "error", "message": "Failed to add todo"}

async def enhance_todo_description(description: str) -> tuple:
    """
    Enhance a todo description using AI, predicting priority and project.
    Returns (enhanced_description, priority, project_name)
    """
    logger.debug(f"Enhancing todo description with AI: {description}")
    
    if HAS_OPENAI and os.environ.get("OPENAI_API_KEY"):
        try:
            # Use OpenAI to enhance the description
            client = openai.AsyncClient()
            
            # Enhance description
            system_prompt = """You are a task enhancement system. Enhance the given task description by:
1. Adding specific technical details
2. Explaining impact and scope
3. Including relevant components/systems
4. Making it more comprehensive
5. Keeping it concise

Output ONLY the enhanced description, no other text."""

            completion = await client.chat.completions.create(
                model="gpt-3.5-turbo",
                messages=[
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": f"Enhance this task: {description}"}
                ]
            )
            enhanced_description = completion.choices[0].message.content.strip()
            
            # Predict priority
            priority_prompt = """You are a task priority classifier. Analyze the task and determine its priority level.
Output ONLY one of these priority levels, with no other text: "critical", "high", "medium", or "low".
Use these guidelines:
- Critical: Must be addressed immediately, major system functionality or security issues
- High: Important tasks that significantly impact functionality or performance
- Medium: Standard development work or minor improvements
- Low: Nice to have features, documentation, or cosmetic issues"""

            completion = await client.chat.completions.create(
                model="gpt-3.5-turbo",
                messages=[
                    {"role": "system", "content": priority_prompt},
                    {"role": "user", "content": f"Classify priority: {description}"}
                ]
            )
            priority_response = completion.choices[0].message.content.strip().lower()
            
            # Map to our priority values
            if priority_response == "critical":
                priority = TaskPriority.CRITICAL
            elif priority_response == "high":
                priority = TaskPriority.HIGH
            elif priority_response == "low":
                priority = TaskPriority.LOW
            else:
                priority = TaskPriority.MEDIUM
                
            # Predict project
            project_prompt = """You are a project classifier. Your task is to determine which project a given task belongs to.
Your output should be ONLY the project name, nothing else.
If you're unsure, respond with "wyrmwatch"."""

            completion = await client.chat.completions.create(
                model="gpt-3.5-turbo",
                messages=[
                    {"role": "system", "content": project_prompt},
                    {"role": "user", "content": f"Which project does this task belong to? {description}"}
                ]
            )
            project = completion.choices[0].message.content.strip()
            
            # Clean up project name
            project = project.strip().strip('"\'').strip()
            project = project if project else "wyrmwatch"
            
            logger.debug("AI enhancement successful!")
            return enhanced_description, priority, project
            
        except Exception as e:
            logger.error(f"Error using OpenAI: {e}")
            # Fall back to simple enhancement
    
    # Simple enhancement if OpenAI not available
    words = description.split()
    if len(words) > 5:
        enhanced = f"Enhanced: {description}\n\nThis task involves working with the wyrmwatch system."
    else:
        enhanced = f"Task: {description}\n\nThis requires attention to detail and careful implementation."
    
    # Simple priority detection based on keywords
    priority = TaskPriority.MEDIUM
    if any(word in description.lower() for word in ["urgent", "critical", "immediately", "asap"]):
        priority = TaskPriority.HIGH
    elif any(word in description.lower() for word in ["minor", "cosmetic", "eventually", "when possible"]):
        priority = TaskPriority.LOW
    
    # Project detection
    project = "wyrmwatch"
    if "slack" in description.lower():
        project = "slack-integration"
    elif "github" in description.lower():
        project = "github-tools"
    
    return enhanced, priority, project

class MqttClient:
    """Handles MQTT communication for WyrmWatch."""
    
    def __init__(self, broker_ip, broker_port, client_id="wyrmwatch_server"):
        """Initialize MQTT client."""
        self.client = mqtt.Client(client_id=client_id, protocol=mqtt.MQTTv5)
        self.broker_ip = broker_ip
        self.broker_port = broker_port
        self.is_connected = False
        self.message_callbacks = []
        
        # Set up callbacks
        self.client.on_connect = self._on_connect
        self.client.on_message = self._on_message
        self.client.on_disconnect = self._on_disconnect
        
    def _on_connect(self, client, userdata, flags, rc, properties=None):
        """Callback for when the client connects to the broker."""
        if rc == 0:
            logger.info(f"Connected to MQTT broker at {self.broker_ip}:{self.broker_port}")
            self.is_connected = True
            
            # Subscribe to topics
            self.client.subscribe("mcp/+", qos=2)
            logger.info("Successfully subscribed to mcp/+")
            
            self.client.subscribe("mcp_server/control", qos=2)
            logger.info("Successfully subscribed to mcp_server/control")
        else:
            logger.error(f"Failed to connect to MQTT broker, return code: {rc}")
    
    def _on_message(self, client, userdata, msg):
        """Callback for when a message is received from the broker."""
        for callback in self.message_callbacks:
            callback(msg.topic, msg.payload)
    
    def _on_disconnect(self, client, userdata, rc):
        """Callback for when the client disconnects from the broker."""
        logger.info(f"Disconnected from MQTT broker with code: {rc}")
        self.is_connected = False
    
    def connect(self):
        """Connect to the MQTT broker."""
        try:
            self.client.connect(self.broker_ip, self.broker_port, keepalive=30)
            self.client.loop_start()
            return True
        except Exception as e:
            logger.error(f"Failed to connect to MQTT broker: {e}")
            return False
    
    def disconnect(self):
        """Disconnect from the MQTT broker."""
        try:
            self.client.loop_stop()
            self.client.disconnect()
            return True
        except Exception as e:
            logger.error(f"Error disconnecting from MQTT broker: {e}")
            return False
    
    def publish(self, topic, payload, qos=2, retain=False):
        """Publish a message to the specified topic."""
        if not self.is_connected:
            logger.error("Cannot publish: not connected to MQTT broker")
            return False
        
        try:
            # Convert dict to JSON string if needed
            if isinstance(payload, dict):
                payload = json.dumps(payload)
                
            # Publish the message
            result = self.client.publish(topic, payload, qos=qos, retain=retain)
            if result.rc != mqtt.MQTT_ERR_SUCCESS:
                logger.error(f"Failed to publish to {topic}: {result.rc}")
                return False
            return True
        except Exception as e:
            logger.error(f"Error publishing to {topic}: {e}")
            return False
    
    def add_message_callback(self, callback):
        """Add a callback function for message handling."""
        self.message_callbacks.append(callback)

class WyrmWatchServer:
    """Main server class for WyrmWatch."""
    
    def __init__(self):
        """Initialize the WyrmWatch server."""
        # Get broker info from environment
        broker_ip = os.environ.get("AWSIP", "localhost")
        broker_port = int(os.environ.get("AWSPORT", "1883"))
        
        # Initialize components
        self.metrics = TaskMetrics()
        self.todo_tool = TodoTool()
        self.mqtt_client = MqttClient(broker_ip, broker_port)
        
        # Create semaphores for rate limiting
        self.task_semaphore = asyncio.Semaphore(MAX_CONCURRENT_TASKS)
        self.ai_semaphore = asyncio.Semaphore(MAX_CONCURRENT_AI)
        
        # For shutdown handling
        self.shutdown_event = asyncio.Event()
        self.mqtt_client.add_message_callback(self.handle_message)
        
        logger.info("WyrmWatch server initialized")
    
    async def start(self):
        """Start the WyrmWatch server."""
        logger.info("Starting WyrmWatch server...")
        
        # Connect to MQTT
        if not self.mqtt_client.connect():
            logger.error("Failed to connect to MQTT. Exiting.")
            return
        
        # Start metrics reporting task
        asyncio.create_task(self.report_metrics())
        
        # Set up signal handlers for graceful shutdown
        for sig in (signal.SIGINT, signal.SIGTERM):
            asyncio.get_event_loop().add_signal_handler(
                sig, lambda s=sig: asyncio.create_task(self.shutdown(s))
            )
        
        logger.info("WyrmWatch server started and listening for tasks")
        
        # Wait for shutdown signal
        await self.shutdown_event.wait()
    
    async def shutdown(self, sig=None):
        """Handle graceful shutdown."""
        if sig:
            logger.info(f"Received shutdown signal: {sig}")
        
        logger.info("Initiating graceful shutdown...")
        
        # Publish final status
        shutdown_payload = {
            "status": "shutdown",
            "timestamp": datetime.now().isoformat(),
            "final_metrics": self.metrics.as_json()
        }
        
        self.mqtt_client.publish(
            "response/mcp_server/status", 
            shutdown_payload
        )
        
        # Disconnect from MQTT
        self.mqtt_client.disconnect()
        
        # Allow time for final messages
        await asyncio.sleep(1)
        
        logger.info("Graceful shutdown complete")
        self.shutdown_event.set()
    
    async def report_metrics(self):
        """Periodically report metrics."""
        while not self.shutdown_event.is_set():
            # Publish metrics
            metrics_json = self.metrics.as_json()
            self.mqtt_client.publish(
                "metrics/response/mcp_todo_server",
                metrics_json
            )
            
            # Wait for the next interval
            try:
                await asyncio.wait_for(
                    self.shutdown_event.wait(),
                    timeout=METRICS_REPORTING_INTERVAL
                )
            except asyncio.TimeoutError:
                # This is expected, continue reporting
                pass
    
    def handle_message(self, topic, payload_bytes):
        """Handle incoming MQTT messages."""
        payload = payload_bytes.decode('utf-8')
        logger.info(f"Received payload on {topic}: {payload}")
        
        # Handle control messages
        if topic == "mcp_server/control":
            try:
                control_json = json.loads(payload)
                command = control_json.get("command")
                
                if command == "shutdown":
                    logger.info("Received shutdown command")
                    asyncio.create_task(self.shutdown())
                    return
                
                elif command == "status":
                    # Report status
                    status_payload = {
                        "status": "running",
                        "timestamp": datetime.now().isoformat(),
                        "metrics": self.metrics.as_json()
                    }
                    
                    self.mqtt_client.publish(
                        "response/mcp_server/status",
                        status_payload
                    )
                    return
            except json.JSONDecodeError:
                logger.error(f"Invalid JSON in control message: {payload}")
        
        # Handle task messages
        if topic.startswith("mcp/"):
            # Increment received counter
            task_count = self.metrics.increment_received()
            logger.debug(f"Task count: {task_count}")
            
            # Extract target agent from topic
            target_agent = topic.split('/')[1] if len(topic.split('/')) > 1 else "user"
            
            # Spawn a task to handle this request
            asyncio.create_task(self.process_task(payload, target_agent))
    
    async def process_task(self, payload, target_agent):
        """Process an incoming task."""
        # Acquire task processing permit
        async with self.task_semaphore:
            logger.debug("Acquired task processing permit")
            
            try:
                # Parse payload
                try:
                    payload_json = json.loads(payload)
                    if isinstance(payload_json, dict) and "description" in payload_json:
                        description = payload_json["description"]
                    else:
                        description = payload
                except json.JSONDecodeError:
                    description = payload
                
                # Prepare parameters for TodoTool
                params = {
                    "command": "add",
                    "description": description,
                    "context": "wyrmwatch_server",
                    "target_agent": target_agent
                }
                
                # Acquire AI enhancement permit
                async with self.ai_semaphore:
                    logger.debug("Acquired AI enhancement permit")
                    
                    # Execute TodoTool command
                    try:
                        result = await self.todo_tool.execute(params)
                        logger.info(f"Successfully added todo: {description}")
                        self.metrics.increment_processed()
                        
                        # Publish success response
                        response_topic = f"response/{target_agent}/todo"
                        response_payload = {
                            "status": "success",
                            "message": result,
                            "timestamp": datetime.now().isoformat()
                        }
                        
                        self.mqtt_client.publish(response_topic, response_payload)
                    
                    except Exception as e:
                        logger.error(f"Failed to add todo: {e}")
                        self.metrics.increment_failed()
                        
                        # Publish error response
                        error_topic = f"response/{target_agent}/error"
                        error_payload = {
                            "status": "error",
                            "error": str(e),
                            "timestamp": datetime.now().isoformat()
                        }
                        
                        self.mqtt_client.publish(error_topic, error_payload)
            
            except Exception as e:
                logger.error(f"Error processing task: {e}")
                self.metrics.increment_failed()

async def main():
    """Main function to run the WyrmWatch server."""
    server = WyrmWatchServer()
    await server.start()

if __name__ == "__main__":
    # Run the server
    asyncio.run(main()) 
