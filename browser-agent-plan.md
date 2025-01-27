Based on analyzing the existing agents in the src/agents directory and reading through the browser-agent README, here are my initial thoughts on integrating vs creating a new browser agent:
The existing agents seem to follow a modular structure, with each agent implemented in its own file and registered in the AgentRegistry. The agents implement the Agent trait which defines a standard interface. This modular design should make it relatively straightforward to integrate a new agent.
The browser-agent project provides a bridge between GPT-4 and a headless Chromium browser. It allows automating browser actions by describing them to the program. The README outlines the installation process, usage instructions, and high-level architecture.
To integrate browser-agent, the main steps would likely be:
1. Create a new BrowserAgent struct that implements the Agent trait
In the BrowserAgent methods, call out to the browser-agent library to execute browser actions
Define a new AgentConfig for the BrowserAgent and register it in AgentRegistry::create_default_agents
Update any relevant tools/executors to interact with the BrowserAgent
Some open questions/challenges with integration:
How to map the browser-agent API to the Agent trait methods
Managing the browser process lifetime within the agent
Handling errors/timeouts from browser interactions
Deciding what configuration the BrowserAgent needs
In contrast, building a new browser agent from scratch would involve:
Choosing a browser automation library (e.g. fantoccini, thirtyfour)
Implementing the browser interaction logic directly in the Agent methods
Managing the browser process within the agent
Handling configuration, errors, logging etc
The tradeoffs seem to be:
Integration leverages existing browser-agent capabilities but requires mapping to the Agent model
New agent has more direct control but requires reimplementing browser automation
My recommendation would be to start by attempting to integrate browser-agent, as it seems to offer a fairly complete set of functionality that we can leverage. If there are significant impedance mismatches with the Agent model, we can fallback to implementing a simplified browser agent directly.
The next steps I would take are:
Prototype a BrowserAgent that wraps browser-agent and implements the Agent trait
Test the integrated agent with a simple workflow
Assess the ergonomics and identify any friction points
Decide whether to continue with integration or pivot to a custom implementation
Let me know if you have any other questions! I'm happy to dive deeper on any part of the analysis or proposed approach.
