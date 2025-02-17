#[cfg(feature = "rl")]
pub mod viz;

use super::{State, Action, Environment};
use std::f64::consts::PI;
use rand::Rng;
use serde::{Serialize, Deserialize};

const GRAVITY: f64 = 0.25;
const FLAP_FORCE: f64 = -4.0;
const BIRD_RADIUS: f64 = 12.0;
const PIPE_WIDTH: f64 = 52.0;
const PIPE_GAP: f64 = 100.0;
const SCREEN_WIDTH: f64 = 288.0;
const SCREEN_HEIGHT: f64 = 512.0;
const MIN_PIPE_HEIGHT: f64 = 50.0;  // Minimum height for pipes
const MAX_PIPE_HEIGHT: f64 = SCREEN_HEIGHT - PIPE_GAP - MIN_PIPE_HEIGHT;  // Maximum height considering gap

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct FlappyBirdState {
    pub(crate) bird_y: i32,
    pub(crate) bird_velocity: i32,
    pub(crate) next_pipe_dist: i32,
    pub(crate) next_pipe_top: i32,
    pub(crate) next_pipe_bottom: i32,
    pub(crate) score: i32,
}

impl Default for FlappyBirdState {
    fn default() -> Self {
        Self {
            bird_y: (SCREEN_HEIGHT / 2.0) as i32,
            bird_velocity: 0,
            next_pipe_dist: SCREEN_WIDTH as i32,
            next_pipe_top: 0,
            next_pipe_bottom: PIPE_GAP as i32,
            score: 0,
        }
    }
}

impl State for FlappyBirdState {
    fn to_features(&self) -> Vec<f64> {
        vec![
            self.bird_y as f64,
            self.bird_velocity as f64,
            self.next_pipe_dist as f64,
            self.next_pipe_top as f64,
            self.next_pipe_bottom as f64,
        ]
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FlappyBirdAction {
    Flap,
    DoNothing,
}

impl Action for FlappyBirdAction {
    fn to_index(&self) -> usize {
        match self {
            FlappyBirdAction::Flap => 0,
            FlappyBirdAction::DoNothing => 1,
        }
    }

    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(FlappyBirdAction::Flap),
            1 => Some(FlappyBirdAction::DoNothing),
            _ => None,
        }
    }
}

pub struct FlappyBirdEnv {
    state: FlappyBirdState,
    frame_iteration: i32,
}

impl Default for FlappyBirdEnv {
    fn default() -> Self {
        Self {
            state: FlappyBirdState::default(),
            frame_iteration: 0,
        }
    }
}

impl FlappyBirdEnv {
    fn check_collision(&self) -> bool {
        // Bird hits the ground or ceiling
        if self.state.bird_y <= 0 || self.state.bird_y >= SCREEN_HEIGHT as i32 {
            return true;
        }

        // Bird hits pipes
        if self.state.next_pipe_dist <= PIPE_WIDTH as i32 && 
           self.state.next_pipe_dist >= -PIPE_WIDTH as i32 {
            if self.state.bird_y <= self.state.next_pipe_top || 
               self.state.bird_y >= self.state.next_pipe_bottom {
                return true;
            }
        }

        false
    }

    pub fn get_score(&self) -> i32 {
        self.state.score
    }

    fn generate_new_pipe(&mut self) {
        let mut rng = rand::thread_rng();
        
        // Generate random height for top pipe
        let top_height = rng.gen_range(MIN_PIPE_HEIGHT as i32..MAX_PIPE_HEIGHT as i32);
        
        self.state.next_pipe_dist = SCREEN_WIDTH as i32;
        self.state.next_pipe_top = top_height;
        self.state.next_pipe_bottom = top_height + PIPE_GAP as i32;
        self.state.score += 1;
    }
}

impl Environment for FlappyBirdEnv {
    type S = FlappyBirdState;
    type A = FlappyBirdAction;

    fn reset(&mut self) -> Self::S {
        self.state = FlappyBirdState::default();
        self.frame_iteration = 0;
        
        // Generate initial pipe
        let mut rng = rand::thread_rng();
        self.state.next_pipe_top = rng.gen_range(MIN_PIPE_HEIGHT as i32..MAX_PIPE_HEIGHT as i32);
        self.state.next_pipe_bottom = self.state.next_pipe_top + PIPE_GAP as i32;
        
        self.state.clone()
    }

    fn step(&mut self, action: &Self::A) -> (Self::S, f64, bool) {
        self.frame_iteration += 1;

        // Apply action
        match action {
            FlappyBirdAction::Flap => {
                self.state.bird_velocity = FLAP_FORCE as i32;
            }
            FlappyBirdAction::DoNothing => {
                // Just let gravity do its thing
            }
        }

        // Update bird position and velocity
        self.state.bird_velocity = (self.state.bird_velocity as f64 + GRAVITY) as i32;
        self.state.bird_y += self.state.bird_velocity;

        // Update pipe position
        self.state.next_pipe_dist -= 2;  // Pipe movement speed
        if self.state.next_pipe_dist <= -PIPE_WIDTH as i32 {
            self.generate_new_pipe();
        }

        // Check for collisions
        let collision = self.check_collision();
        
        // Calculate reward
        let reward = if collision {
            -10.0  // Big penalty for collision
        } else if self.state.next_pipe_dist == (SCREEN_WIDTH/2.0) as i32 {
            10.0   // Reward for passing pipe
        } else {
            // Improved reward shaping
            let dist_reward = 1.0 - (self.state.next_pipe_dist.abs() as f64 / SCREEN_WIDTH);
            let height_diff = (self.state.bird_y - ((self.state.next_pipe_top + self.state.next_pipe_bottom) / 2)) as f64;
            let height_reward = 1.0 - (height_diff.abs() / SCREEN_HEIGHT);
            let velocity_penalty = (self.state.bird_velocity.abs() as f64 / 10.0).min(1.0);  // Penalize high velocities
            (dist_reward + height_reward) / 2.0 - velocity_penalty * 0.2  // Small reward for staying alive and being in good position
        };

        // Check terminal conditions
        let done = collision || 
                  self.frame_iteration > 1000 || // Prevent infinite episodes
                  self.state.bird_y <= 0 || 
                  self.state.bird_y >= SCREEN_HEIGHT as i32;

        (self.state.clone(), reward, done)
    }

    fn action_space_size(&self) -> usize {
        2
    }

    fn valid_actions(&self, _state: &Self::S) -> Vec<Self::A> {
        vec![FlappyBirdAction::Flap, FlappyBirdAction::DoNothing]
    }
}

#[cfg(test)]
#[cfg(feature = "rl")]
mod tests {
    use super::*;
    use crate::agents::rl::QLearningAgent;

    #[test]
    fn test_flappy_bird_env() {
        let mut env = FlappyBirdEnv::default();
        let mut agent = QLearningAgent::new(0.1, 0.95, 0.1);
        
        let initial_state = env.reset();
        assert_eq!(initial_state.bird_y, (SCREEN_HEIGHT / 2.0) as i32);
        
        // Test a few steps
        let valid_actions = env.valid_actions(&initial_state);
        let action = agent.choose_action(&initial_state, &valid_actions);
        let (next_state, reward, done) = env.step(&action);
        
        // Verify state changes
        assert!(next_state.bird_y != initial_state.bird_y, "Bird position should change");
        assert!(next_state.next_pipe_dist < initial_state.next_pipe_dist, "Pipe should move closer");
    }
} 
