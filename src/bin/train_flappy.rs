#[cfg(feature = "rl")]
use swarmonomicon::agents::rl::{
    Environment,
    QLearningAgent,
    flappy::{FlappyBirdEnv, FlappyBirdState, FlappyBirdAction},
};
#[cfg(feature = "rl")]
use swarmonomicon::agents::rl::flappy::viz::FlappyViz;
use std::time::Duration;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

const TRAINING_EPISODES: i32 = 1000;
const RENDER_EVERY_N_EPISODES: i32 = 1;
const SAVE_EVERY_N_EPISODES: i32 = 100;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = FlappyBirdEnv::default();
    let mut agent = QLearningAgent::new(0.1, 0.99, 0.1);

    // Initialize visualization
    let (mut viz, event_loop) = FlappyViz::new()?;

    let mut episode = 0;
    let mut best_score = 0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::MainEventsCleared => {
                // Run training loop
                if episode < TRAINING_EPISODES {
                    if episode % RENDER_EVERY_N_EPISODES == 0 {
                        // Run one step with visualization
                        if let Err(e) = run_training_step(&mut env, &mut agent, Some(&mut viz)) {
                            eprintln!("Error during training: {}", e);
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    } else {
                        // Run one step without visualization
                        if let Err(e) = run_training_step(&mut env, &mut agent, None) {
                            eprintln!("Error during training: {}", e);
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }

                    // Update best score
                    if env.get_score() > best_score {
                        best_score = env.get_score();
                        println!("New best score: {}", best_score);
                    }

                    // Save progress periodically
                    if episode % SAVE_EVERY_N_EPISODES == 0 {
                        println!("Episode {}: Score = {}, Best = {}", episode, env.get_score(), best_score);
                    }

                    episode += 1;
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    })
}

fn run_training_step(
    env: &mut FlappyBirdEnv,
    agent: &mut QLearningAgent<FlappyBirdState, FlappyBirdAction>,
    mut viz: Option<&mut FlappyViz>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = env.reset();
    let mut total_reward = 0.0;

    loop {
        // Get action from agent
        let valid_actions = env.valid_actions(&state);
        let action = agent.choose_action(&state, &valid_actions);

        // Take step in environment
        let (next_state, reward, done) = env.step(&action);
        total_reward += reward;

        // Visualize if requested
        if let Some(viz) = viz.as_mut() {
            viz.render(&next_state)?;
            std::thread::sleep(Duration::from_millis(16)); // Cap at ~60 FPS
        }

        // Learn from experience
        let next_valid_actions = env.valid_actions(&next_state);
        agent.learn(state.clone(), action, reward, &next_state, &next_valid_actions);

        if done {
            break;
        }
        state = next_state;
    }

    Ok(())
}
