#![cfg(feature = "rl")]

use clap::Parser;
use swarmonomicon::agents::rl::{
    Environment,
    flappy::{FlappyBirdEnv, FlappyBirdState, FlappyBirdAction, viz::FlappyViz},
    QLearningAgent,
};
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::WindowBuilder;
use winit::event::Event;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to save/load the model
    #[arg(short, long)]
    model_path: Option<PathBuf>,

    /// Whether to visualize the training
    #[arg(short, long)]
    visualize: bool,

    /// Number of episodes to train
    #[arg(short, long, default_value = "1000")]
    episodes: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let model_path = args.model_path.unwrap_or_else(|| PathBuf::from("flappy_model.json"));

    let mut env = FlappyBirdEnv::default();
    let mut agent = if model_path.exists() {
        let mut agent = QLearningAgent::new(0.1, 0.95, 0.1);
        agent.load_model(&model_path).await?;
        agent
    } else {
        QLearningAgent::new(0.1, 0.95, 0.1)
    };

    let (event_loop, window, mut viz) = if args.visualize {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Flappy Bird Training")
            .with_inner_size(winit::dpi::LogicalSize::new(288.0, 512.0))
            .build(&event_loop)
            .unwrap();
        let viz = FlappyViz::new(&window);
        (Some(event_loop), Some(window), Some(viz))
    } else {
        (None, None, None)
    };

    let mut best_score = 0;
    let mut total_reward = 0.0;
    let target_fps = 60.0;
    let frame_time = Duration::from_secs_f64(1.0 / target_fps);

    if let (Some(event_loop), Some(_window), Some(ref mut viz)) = (&event_loop, &window, &mut viz) {
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::MainEventsCleared => {
                    let frame_start = Instant::now();
                    
                    // Training loop
                    let state = env.reset();
                    let mut done = false;
                    let mut episode_reward = 0.0;

                    while !done {
                        let valid_actions = env.valid_actions(&state);
                        let action = agent.choose_action(&state, &valid_actions);
                        let (next_state, reward, is_done) = env.step(&action);
                        
                        agent.update(&state, &action, reward, &next_state);
                        episode_reward += reward;
                        done = is_done;

                        // Update visualization
                        viz.render(&next_state);

                        // Maintain frame rate
                        let elapsed = frame_start.elapsed();
                        if elapsed < frame_time {
                            std::thread::sleep(frame_time - elapsed);
                        }
                    }

                    total_reward += episode_reward;
                    let score = env.get_score();
                    if score > best_score {
                        best_score = score;
                        println!("New best score: {}", best_score);
                    }
                }
                Event::WindowEvent { event: winit::event::WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => (),
            }
        });
    } else {
        // Training without visualization
        for episode in 0..args.episodes {
            let state = env.reset();
            let mut done = false;
            let mut episode_reward = 0.0;

            while !done {
                let valid_actions = env.valid_actions(&state);
                let action = agent.choose_action(&state, &valid_actions);
                let (next_state, reward, is_done) = env.step(&action);
                
                agent.update(&state, &action, reward, &next_state);
                episode_reward += reward;
                done = is_done;
            }

            total_reward += episode_reward;
            let score = env.get_score();
            if score > best_score {
                best_score = score;
                println!("New best score: {}", best_score);
            }

            if (episode + 1) % 100 == 0 {
                println!("Episode {}/{}, Average Reward: {:.2}, Best Score: {}", 
                    episode + 1, args.episodes, total_reward / (episode + 1) as f64, best_score);
            }
        }
    }

    // Save the trained model
    agent.save_model(&model_path).await?;
    println!("Model saved to {:?}", model_path);
    println!("Training completed. Best score: {}", best_score);

    Ok(())
}

#[cfg(not(feature = "rl"))]
fn main() {
    println!("This binary requires the 'rl' feature to be enabled.");
    println!("Please rebuild with: cargo build --features rl");
    std::process::exit(1);
} 
