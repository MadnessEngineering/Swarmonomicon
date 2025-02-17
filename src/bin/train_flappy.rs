#![cfg(feature = "rl")]

use clap::Parser;
use swarmonomicon::agents::rl::{
    Environment,
    flappy::{FlappyBirdEnv, FlappyBirdState, FlappyBirdAction, viz::FlappyViz},
    QLearningAgent,
};
use anyhow::{Result, anyhow};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to save/load the model
    #[arg(short, long)]
    model: Option<PathBuf>,

    /// Number of episodes to train
    #[arg(short, long, default_value = "1000")]
    episodes: i32,

    /// Learning rate
    #[arg(short, long, default_value = "0.1")]
    learning_rate: f64,

    /// Discount factor
    #[arg(short, long, default_value = "0.95")]
    discount_factor: f64,

    /// Exploration rate (epsilon)
    #[arg(short, long, default_value = "0.1")]
    epsilon: f64,

    /// Whether to visualize the training
    #[arg(short, long)]
    visualize: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // Initialize environment and agent
    let mut env = FlappyBirdEnv::default();
    let mut agent = if let Some(ref model_path) = args.model {
        if model_path.exists() {
            println!("Loading model from {}", model_path.display());
            QLearningAgent::load_model(&model_path)
                .map_err(|e| anyhow!("Failed to load model: {}", e))?
        } else {
            println!("Creating new model that will be saved to {}", model_path.display());
            QLearningAgent::new(args.learning_rate, args.discount_factor, args.epsilon)
        }
    } else {
        QLearningAgent::new(args.learning_rate, args.discount_factor, args.epsilon)
    };

    // Initialize visualization if requested
    let mut viz = if args.visualize {
        let (viz, event_loop) = FlappyViz::new()
            .map_err(|e| anyhow!("Failed to initialize visualization: {}", e))?;
        Some((viz, event_loop))
    } else {
        None
    };

    let mut best_score = 0;
    let mut total_reward = 0.0;

    println!("Starting training for {} episodes...", args.episodes);
    for episode in 0..args.episodes {
        let mut state = env.reset();
        let mut episode_reward = 0.0;

        loop {
            let valid_actions = env.valid_actions(&state);
            let action = agent.choose_action(&state, &valid_actions);
            let (next_state, reward, done) = env.step(&action);

            episode_reward += reward;

            // Update the agent
            let next_valid_actions = env.valid_actions(&next_state);
            agent.learn(state.clone(), action, reward, &next_state, &next_valid_actions);

            // Visualize if requested
            if let Some((viz, _)) = &mut viz {
                viz.render(&next_state)
                    .map_err(|e| anyhow!("Visualization error: {}", e))?;
            }

            if done {
                break;
            }
            state = next_state;
        }

        total_reward += episode_reward;
        let score = env.get_score();
        if score > best_score {
            best_score = score;
        }

        if (episode + 1) % 100 == 0 {
            println!(
                "Episode {}/{}: Score = {}, Best = {}, Avg Reward = {:.2}",
                episode + 1,
                args.episodes,
                score,
                best_score,
                total_reward / (episode + 1) as f64
            );

            // Save model if path was provided
            if let Some(model_path) = &args.model {
                agent.save_model(model_path)
                    .map_err(|e| anyhow!("Failed to save model: {}", e))?;
                println!("Model saved to {}", model_path.display());
            }
        }
    }

    println!("\nTraining completed!");
    println!("Best score: {}", best_score);
    println!("Average reward: {:.2}", total_reward / args.episodes as f64);

    Ok(())
}

#[cfg(not(feature = "rl"))]
fn main() {
    println!("This binary requires the 'rl' feature to be enabled.");
    println!("Please rebuild with: cargo build --features rl");
    std::process::exit(1);
} 
