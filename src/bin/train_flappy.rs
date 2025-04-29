#![cfg(feature = "rl")]

use clap::Parser;
use swarmonomicon::agents::rl::{
    Environment,
    flappy::{FlappyBirdEnv, FlappyBirdState, FlappyBirdAction, viz::FlappyViz},
    model::config::{TrainingConfig, TrainingMetrics, TrainingHistory},
    viz::VisualizationTools,
    QLearningAgent,
};
use anyhow::Result;
use std::path::{PathBuf, Path};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::WindowBuilder;
use winit::event::Event;
use std::time::{Duration, Instant};
use std::fs;
use std::sync::{Arc, Mutex};
use std::io::{self, Write};

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

    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Path to save metrics and visualizations
    #[arg(short = 'm', long)]
    metrics_path: Option<PathBuf>,
    
    /// Resume training from the latest checkpoint
    #[arg(short, long)]
    resume: bool,
    
    /// Number of latest checkpoints to keep
    #[arg(long, default_value = "5")]
    keep_checkpoints: usize,
    
    /// Keep checkpoints at this episode interval
    #[arg(long, default_value = "100")]
    checkpoint_interval: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Load or create config
    let mut config = if let Some(config_path) = &args.config {
        if config_path.exists() {
            TrainingConfig::load(config_path)?
        } else {
            let config = TrainingConfig::default();
            config.save(config_path)?;
            config
        }
    } else {
        TrainingConfig::default()
    };
    
    // Override config with command line args
    config.episodes = args.episodes;
    config.visualize = args.visualize;
    if let Some(path) = &args.model_path {
        config.checkpoint_path = path.to_string_lossy().to_string();
    }
    if let Some(path) = &args.metrics_path {
        config.metrics_path = path.to_string_lossy().to_string();
        config.save_metrics = true;
    }
    
    // Ensure directories exist
    fs::create_dir_all(&config.checkpoint_path).unwrap_or_default();
    if config.save_metrics {
        fs::create_dir_all(&config.metrics_path).unwrap_or_default();
    }

    // Setup model paths
    let checkpoint_dir = PathBuf::from(&config.checkpoint_path);
    let model_path = checkpoint_dir.join("flappy_model.json");
    
    // Training history and tracking variables
    let mut starting_episode = 0;
    let mut history = TrainingHistory::new(config.clone());
    let mut best_score = 0;
    
    // Initialize environment and agent
    let env = Arc::new(Mutex::new(FlappyBirdEnv::default()));
    let agent = Arc::new(Mutex::new({
        if args.resume {
            // Try to load the latest checkpoint
            println!("Attempting to resume from latest checkpoint...");
            match QLearningAgent::<FlappyBirdState, FlappyBirdAction>::load_latest_checkpoint(&checkpoint_dir).await {
                Ok(Some(mut agent)) => {
                    // Extract training progress from the loaded model
                    starting_episode = agent.metadata.episodes_trained;
                    best_score = agent.metadata.best_score as i32;
                    println!("Resuming from episode {}, best score: {}", starting_episode, best_score);
                    
                    // Also try to load the training history
                    let history_path = PathBuf::from(&config.metrics_path).join("training_history.json");
                    if history_path.exists() {
                        match TrainingHistory::load(&history_path) {
                            Ok(loaded_history) => {
                                history = loaded_history;
                                println!("Loaded training history with {} metrics entries", history.metrics.len());
                            },
                            Err(e) => println!("Failed to load training history: {}", e),
                        }
                    }
                    
                    agent
                }
                Ok(None) => {
                    println!("No checkpoint found. Starting new training.");
                    QLearningAgent::new(
                        config.learning_rate,
                        config.discount_factor,
                        config.epsilon,
                    )
                }
                Err(e) => {
                    println!("Error loading checkpoint: {}. Starting new training.", e);
                    QLearningAgent::new(
                        config.learning_rate,
                        config.discount_factor,
                        config.epsilon,
                    )
                }
            }
        } else if model_path.exists() {
            // Load from specific model path
            println!("Loading model from: {:?}", model_path);
            let mut agent = QLearningAgent::new(
                config.learning_rate,
                config.discount_factor,
                config.epsilon,
            );
            match agent.load_model(&model_path).await {
                Ok(_) => {
                    println!("Model loaded successfully");
                    agent
                }
                Err(e) => {
                    println!("Error loading model: {}. Starting new training.", e);
                    QLearningAgent::new(
                        config.learning_rate,
                        config.discount_factor,
                        config.epsilon,
                    )
                }
            }
        } else {
            // Create new agent
            println!("Starting new training.");
            QLearningAgent::new(
                config.learning_rate,
                config.discount_factor,
                config.epsilon,
            )
        }
    }));

    // Create visualization tools if metrics are enabled
    let viz_tools = if config.save_metrics {
        Some(VisualizationTools::new(&config.metrics_path))
    } else {
        None
    };

    // Setup graceful shutdown handling
    let running = Arc::new(Mutex::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, saving checkpoint before exiting...");
        let mut running = r.lock().unwrap();
        *running = false;
    }).expect("Error setting Ctrl+C handler");

    if config.visualize {
        // Training with visualization
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Flappy Bird Training")
            .with_inner_size(winit::dpi::LogicalSize::new(288.0, 512.0))
            .build(&event_loop)
            .unwrap();
        let mut viz = FlappyViz::new(&window);
        
        let agent_clone = agent.clone();
        let env_clone = env.clone();
        let history_clone = Arc::new(Mutex::new(history));
        let config_clone = config.clone();
        let checkpoint_dir_clone = checkpoint_dir.clone();
        let viz_tools_clone = viz_tools.clone();
        let running_clone = running.clone();
        
        let mut current_episode = starting_episode;
        
        event_loop.run(move |event, _, control_flow| {
            // Check for shutdown signal
            if !*running_clone.lock().unwrap() {
                // Save final checkpoint
                let final_checkpoint_path = checkpoint_dir_clone.join("final_checkpoint.json");
                futures::executor::block_on(async {
                    let agent = agent_clone.lock().unwrap();
                    if let Err(e) = agent.save_model(&final_checkpoint_path).await {
                        eprintln!("Error saving final checkpoint: {}", e);
                    } else {
                        println!("Final checkpoint saved at {:?}", final_checkpoint_path);
                    }
                });
                
                // Save training history
                let history_path = PathBuf::from(&config_clone.metrics_path).join("training_history.json");
                let history = history_clone.lock().unwrap();
                if let Err(e) = history.save(&history_path) {
                    eprintln!("Error saving training history: {}", e);
                } else {
                    println!("Training history saved at {:?}", history_path);
                }
                
                *control_flow = ControlFlow::Exit;
                return;
            }
        
            *control_flow = ControlFlow::Poll;

            match event {
                Event::MainEventsCleared => {
                    if current_episode >= config_clone.episodes {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    
                    let frame_start = Instant::now();
                    
                    // Training loop for one episode
                    let mut env = env_clone.lock().unwrap();
                    let state = env.reset();
                    let mut current_state = state;
                    let mut done = false;
                    let mut episode_reward = 0.0;
                    let mut steps = 0;

                    while !done {
                        let valid_actions = env.valid_actions(&current_state);
                        let action = {
                            let mut agent = agent_clone.lock().unwrap();
                            agent.choose_action(&current_state, &valid_actions)
                        };
                        let (next_state, reward, is_done) = env.step(&action);
                        
                        {
                            let mut agent = agent_clone.lock().unwrap();
                            agent.update(&current_state, &action, reward, &next_state);
                        }
                        
                        episode_reward += reward;
                        done = is_done;
                        current_state = next_state.clone();
                        steps += 1;

                        // Update visualization
                        viz.render(&next_state);

                        // Maintain frame rate
                        let elapsed = frame_start.elapsed();
                        if elapsed < frame_time {
                            std::thread::sleep(frame_time - elapsed);
                        }
                    }

                    let score = env.get_score();
                    let is_best = score > best_score;
                    if is_best {
                        best_score = score;
                        println!("Episode {}: New best score: {}", current_episode, best_score);
                    } else if current_episode % 10 == 0 {
                        print!("\rEpisode: {}/{}, Score: {}, Best: {}", 
                              current_episode, config_clone.episodes, score, best_score);
                        io::stdout().flush().unwrap();
                    }
                    
                    total_reward += episode_reward;
                    
                    // Decay epsilon
                    {
                        let mut agent = agent_clone.lock().unwrap();
                        agent.decay_epsilon(&config_clone);
                    }
                    
                    // Record metrics
                    if config_clone.save_metrics {
                        let avg_q = {
                            let agent = agent_clone.lock().unwrap();
                            agent.calculate_avg_q_value(&current_state)
                        };
                        
                        let epsilon = {
                            let agent = agent_clone.lock().unwrap();
                            agent.get_config().epsilon
                        };
                        
                        let metrics = TrainingMetrics {
                            episode: current_episode,
                            reward: episode_reward,
                            score,
                            steps,
                            epsilon,
                            avg_q_value: avg_q,
                        };
                        
                        {
                            let mut history = history_clone.lock().unwrap();
                            history.add_metrics(metrics);
                        }
                    }
                    
                    // Checkpoint if needed
                    if current_episode % config_clone.checkpoint_freq == 0 || is_best {
                        // Clone what we need for the checkpoint
                        let agent_for_save = agent_clone.clone();
                        let checkpoint_dir_for_save = checkpoint_dir_clone.clone();
                        let history_for_report = history_clone.clone();
                        let viz_tools_for_report = viz_tools_clone.clone();
                        
                        // Use a blocking thread to avoid disrupting the event loop
                        std::thread::spawn(move || {
                            // Update agent metadata
                            let mut agent = agent_for_save.lock().unwrap();
                            agent.update_metadata(
                                Some(current_episode),
                                Some(best_score as f64),
                                None
                            );
                            
                            // Save the checkpoint
                            futures::executor::block_on(async {
                                match agent.save_checkpoint(&checkpoint_dir_for_save, current_episode, is_best).await {
                                    Ok(path) => println!("\nCheckpoint saved at {:?}", path),
                                    Err(e) => eprintln!("\nError saving checkpoint: {}", e),
                                }
                            });
                            
                            // Clean up old checkpoints
                            match QLearningAgent::<FlappyBirdState, FlappyBirdAction>::clean_old_checkpoints(
                                &checkpoint_dir_for_save, 
                                args.keep_checkpoints, 
                                Some(args.checkpoint_interval)
                            ) {
                                Ok(deleted) => {
                                    if deleted > 0 {
                                        println!("Cleaned up {} old checkpoints", deleted);
                                    }
                                },
                                Err(e) => eprintln!("Error cleaning old checkpoints: {}", e),
                            }
                            
                            // Generate a report if metrics are enabled
                            if let Some(viz_tools) = viz_tools_for_report {
                                let history = history_for_report.lock().unwrap();
                                if let Ok(report_path) = viz_tools.generate_report(&history) {
                                    println!("Training report generated at {:?}", report_path);
                                }
                                
                                // Save the training history
                                let history_path = PathBuf::from(&config_clone.metrics_path).join("training_history.json");
                                if let Err(e) = history.save(&history_path) {
                                    eprintln!("Error saving training history: {}", e);
                                }
                            }
                        });
                    }
                    
                    current_episode += 1;
                },
                Event::WindowEvent { 
                    event: winit::event::WindowEvent::CloseRequested, .. 
                } => {
                    *control_flow = ControlFlow::Exit;
                },
                _ => {}
            }
        });
    } else {
        // Command-line training without visualization
        let target_fps = 60.0;
        let frame_time = Duration::from_secs_f64(1.0 / target_fps);
        let mut total_reward = 0.0;
        
        let episodes_range = starting_episode..config.episodes;
        let progress_step = config.episodes / 100;
        let progress_step = if progress_step == 0 { 1 } else { progress_step };
        
        println!("Starting training for {} episodes (from episode {})", config.episodes - starting_episode, starting_episode);
        
        for current_episode in episodes_range {
            // Check for shutdown signal
            if !*running.lock().unwrap() {
                break;
            }
            
            // Training loop for one episode
            let mut env = env.lock().unwrap();
            let state = env.reset();
            let mut current_state = state;
            let mut done = false;
            let mut episode_reward = 0.0;
            let mut steps = 0;

            while !done {
                let valid_actions = env.valid_actions(&current_state);
                let action = {
                    let mut agent = agent.lock().unwrap();
                    agent.choose_action(&current_state, &valid_actions)
                };
                let (next_state, reward, is_done) = env.step(&action);
                
                {
                    let mut agent = agent.lock().unwrap();
                    agent.update(&current_state, &action, reward, &next_state);
                }
                
                episode_reward += reward;
                done = is_done;
                current_state = next_state.clone();
                steps += 1;

                // Sleep to prevent CPU overuse
                std::thread::sleep(Duration::from_millis(1));
            }

            let score = env.get_score();
            let is_best = score > best_score;
            if is_best {
                best_score = score;
                println!("\nEpisode {}: New best score: {}", current_episode, best_score);
            } else if current_episode % progress_step == 0 || current_episode % 10 == 0 {
                let progress = (current_episode as f64 / config.episodes as f64) * 100.0;
                print!("\rProgress: {:.1}% - Episode: {}/{}, Score: {}, Best: {}", 
                      progress, current_episode, config.episodes, score, best_score);
                io::stdout().flush().unwrap();
            }
            
            total_reward += episode_reward;
            
            // Decay epsilon
            {
                let mut agent = agent.lock().unwrap();
                agent.decay_epsilon(&config);
            }
            
            // Record metrics
            if config.save_metrics {
                let avg_q = {
                    let agent = agent.lock().unwrap();
                    agent.calculate_avg_q_value(&current_state)
                };
                
                let epsilon = {
                    let agent = agent.lock().unwrap();
                    agent.get_config().epsilon
                };
                
                let metrics = TrainingMetrics {
                    episode: current_episode,
                    reward: episode_reward,
                    score,
                    steps,
                    epsilon,
                    avg_q_value: avg_q,
                };
                
                history.add_metrics(metrics);
            }
            
            // Checkpoint if needed
            if current_episode % config.checkpoint_freq == 0 || is_best {
                // Update agent metadata
                {
                    let mut agent_lock = agent.lock().unwrap();
                    agent_lock.update_metadata(
                        Some(current_episode),
                        Some(best_score as f64),
                        None
                    );
                    
                    // Save the checkpoint
                    match agent_lock.save_checkpoint(&checkpoint_dir, current_episode, is_best).await {
                        Ok(path) => println!("\nCheckpoint saved at {:?}", path),
                        Err(e) => eprintln!("\nError saving checkpoint: {}", e),
                    }
                }
                
                // Clean up old checkpoints
                match QLearningAgent::<FlappyBirdState, FlappyBirdAction>::clean_old_checkpoints(
                    &checkpoint_dir, 
                    args.keep_checkpoints, 
                    Some(args.checkpoint_interval)
                ) {
                    Ok(deleted) => {
                        if deleted > 0 {
                            println!("Cleaned up {} old checkpoints", deleted);
                        }
                    },
                    Err(e) => eprintln!("Error cleaning old checkpoints: {}", e),
                }
                
                // Generate a report if metrics are enabled
                if let Some(viz_tools) = &viz_tools {
                    if let Ok(report_path) = viz_tools.generate_report(&history) {
                        println!("Training report generated at {:?}", report_path);
                    }
                    
                    // Save the training history
                    let history_path = PathBuf::from(&config.metrics_path).join("training_history.json");
                    if let Err(e) = history.save(&history_path) {
                        eprintln!("Error saving training history: {}", e);
                    }
                }
            }
        }
        
        // Save final model
        let final_model_path = checkpoint_dir.join("final_model.json");
        {
            let agent_lock = agent.lock().unwrap();
            match agent_lock.save_model(&final_model_path).await {
                Ok(_) => println!("\nFinal model saved at {:?}", final_model_path),
                Err(e) => eprintln!("\nError saving final model: {}", e),
            }
        }
        
        // Save final training history
        if config.save_metrics {
            let history_path = PathBuf::from(&config.metrics_path).join("training_history.json");
            if let Err(e) = history.save(&history_path) {
                eprintln!("Error saving training history: {}", e);
            } else {
                println!("Training history saved at {:?}", history_path);
            }
        }
        
        println!("\nTraining complete. Total episodes: {}, Best score: {}", config.episodes, best_score);
    }

    Ok(())
}

#[cfg(not(feature = "rl"))]
fn main() {
    println!("This binary requires the 'rl' feature to be enabled.");
    println!("Please rebuild with: cargo build --features rl");
    std::process::exit(1);
} 
