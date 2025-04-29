# WyrmWatch

A mad tinker's monitoring system for tasks and tiny wyrms.

## Overview

WyrmWatch is a Python implementation of the MCP Todo Server concept from the Swarmonomicon project. Named after a fantasy creature that a tinker from J.S. Morin's Black Ocean series might keep as a pet, this server processes todo tasks via MQTT messaging, enhances them with AI, and stores them in MongoDB.

## Features

- **MQTT-based Task Processing**: Listens on `mcp/+` topics for incoming tasks
- **AI Enhancement**: Improves task descriptions, predicts priority and project
- **MongoDB Storage**: Persists tasks with proper indexing
- **Concurrent Processing**: Handles multiple tasks with configurable limits
- **Topic Separation**: Uses different topics for commands vs. responses to prevent loops
- **Metrics Reporting**: Monitors and reports on system performance
- **Graceful Shutdown**: Ensures clean handling of interruptions

## Process Flow

1. **Initialization**:
   - Sets up logging, metrics, database connection
   - Establishes MQTT connection with broker
   - Creates semaphores for concurrency control

2. **Message Reception**:
   - Subscribes to `mcp/+` for incoming commands 
   - Parses messages and extracts task descriptions

3. **Task Processing**:
   - Acquires semaphore permits for processing
   - Enhances task descriptions with AI
   - Stores tasks in MongoDB
   - Publishes responses to separate topics

4. **Response Handling**:
   - Sends success/error responses to `response/{agent}/todo` topics
   - Publishes metrics to `metrics/response/mcp_todo_server`

5. **Shutdown Process**:
   - Publishes final metrics and status
   - Disconnects cleanly from MQTT
   - Releases resources

## Installation

1. Clone this repository
2. Install dependencies:
   ```
   pip install -r requirements.txt
   ```
3. Set environment variables:
   ```
   export MONGODB_URI=mongodb://localhost:27017/
   export AWSIP=localhost  # MQTT broker IP
   export AWSPORT=1883     # MQTT broker port
   ```
4. Run the server:
   ```
   python wyrmwatch_server.py
   ```

## AI Enhancement

The system supports two modes for task enhancement:

1. **OpenAI Mode**: Uses GPT models when `OPENAI_API_KEY` is available
2. **Simple Mode**: Falls back to keyword-based enhancement when OpenAI is unavailable

## MQTT Topic Structure

- `mcp/+` - Topics for incoming commands 
- `response/+/todo` - Success response topics
- `response/+/error` - Error response topics
- `response/mcp_server/status` - Server status responses
- `metrics/response/mcp_todo_server` - Metrics reporting

## Configuration

Adjust these constants in the code to configure behavior:
- `MAX_CONCURRENT_TASKS` (default: 5)
- `MAX_CONCURRENT_AI` (default: 2)
- `METRICS_REPORTING_INTERVAL` (default: 30 seconds)

---

*"Keep an eye on your wyrm, or it might just grow into a problem."* — Old Tinker's saying 
