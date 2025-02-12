#[cfg(feature = "rl")]
use pixels::{Pixels, SurfaceTexture};
#[cfg(feature = "rl")]
use winit::window::WindowBuilder;
#[cfg(feature = "rl")]
use winit::event_loop::EventLoop;
#[cfg(feature = "rl")]
use winit_input_helper::WinitInputHelper;
#[cfg(feature = "rl")]
use std::time::Instant;
#[cfg(feature = "rl")]
use super::FlappyBirdState;

#[cfg(feature = "rl")]
const SCREEN_WIDTH: u32 = 288;
#[cfg(feature = "rl")]
const SCREEN_HEIGHT: u32 = 512;

#[cfg(feature = "rl")]
pub struct FlappyViz {
    pixels: Pixels,
    input: WinitInputHelper,
    last_frame: Instant,
}

#[cfg(feature = "rl")]
impl FlappyViz {
    pub fn new() -> Result<(Self, EventLoop<()>), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new();
        let input = WinitInputHelper::new();
        
        let window = WindowBuilder::new()
            .with_title("Flappy Bird RL")
            .with_inner_size(winit::dpi::LogicalSize::new(SCREEN_WIDTH, SCREEN_HEIGHT))
            .build(&event_loop)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture)?;

        Ok((Self {
            pixels,
            input,
            last_frame: Instant::now(),
        }, event_loop))
    }

    pub fn render(&mut self, state: &FlappyBirdState) -> Result<(), Box<dyn std::error::Error>> {
        // Clear screen
        {
            let frame = self.pixels.frame_mut();
            for pixel in frame.chunks_exact_mut(4) {
                pixel[0] = 135; // R
                pixel[1] = 206; // G
                pixel[2] = 235; // B
                pixel[3] = 255; // A
            }
        }

        // Draw game elements
        self.draw_bird(state.bird_y as u32);
        self.draw_pipes(state.next_pipe_dist as i32, state.next_pipe_top as u32, state.next_pipe_bottom as u32);
        self.draw_score(state.score);

        self.pixels.render()?;
        
        // Cap at ~60 FPS
        while self.last_frame.elapsed().as_millis() < 16 {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        self.last_frame = Instant::now();

        Ok(())
    }

    fn draw_bird(&mut self, y: u32) {
        const BIRD_SIZE: u32 = 20;
        const BIRD_X: u32 = 50;

        let frame = self.pixels.frame_mut();
        for dy in 0..BIRD_SIZE {
            for dx in 0..BIRD_SIZE {
                let x = BIRD_X + dx;
                let y = y + dy;
                if y < SCREEN_HEIGHT && x < SCREEN_WIDTH {
                    let idx = (y * SCREEN_WIDTH + x) as usize * 4;
                    frame[idx] = 255;     // R
                    frame[idx + 1] = 255; // G
                    frame[idx + 2] = 0;   // B
                    frame[idx + 3] = 255; // A
                }
            }
        }
    }

    fn draw_pipes(&mut self, dist: i32, top: u32, bottom: u32) {
        const PIPE_WIDTH: u32 = 52;
        let pipe_x = (SCREEN_WIDTH as i32 - dist) as u32;

        let frame = self.pixels.frame_mut();
        // Draw top pipe
        for y in 0..top {
            for x in pipe_x..pipe_x.saturating_add(PIPE_WIDTH).min(SCREEN_WIDTH) {
                let idx = (y * SCREEN_WIDTH + x) as usize * 4;
                frame[idx] = 0;     // R
                frame[idx + 1] = 255; // G
                frame[idx + 2] = 0;   // B
                frame[idx + 3] = 255; // A
            }
        }

        // Draw bottom pipe
        for y in bottom..SCREEN_HEIGHT {
            for x in pipe_x..pipe_x.saturating_add(PIPE_WIDTH).min(SCREEN_WIDTH) {
                let idx = (y * SCREEN_WIDTH + x) as usize * 4;
                frame[idx] = 0;     // R
                frame[idx + 1] = 255; // G
                frame[idx + 2] = 0;   // B
                frame[idx + 3] = 255; // A
            }
        }
    }

    fn draw_score(&mut self, score: i32) {
        let score_str = score.to_string();
        let x = 10;
        let y = 10;
        
        for (i, c) in score_str.chars().enumerate() {
            self.draw_digit(c, x + i as u32 * 20, y);
        }
    }

    fn draw_digit(&mut self, digit: char, x: u32, y: u32) {
        const DIGIT_WIDTH: u32 = 15;
        const DIGIT_HEIGHT: u32 = 20;

        let frame = self.pixels.frame_mut();
        for dy in 0..DIGIT_HEIGHT {
            for dx in 0..DIGIT_WIDTH {
                if x + dx < SCREEN_WIDTH && y + dy < SCREEN_HEIGHT {
                    let idx = ((y + dy) * SCREEN_WIDTH + (x + dx)) as usize * 4;
                    frame[idx] = 255;     // R
                    frame[idx + 1] = 255; // G
                    frame[idx + 2] = 255; // B
                    frame[idx + 3] = 255; // A
                }
            }
        }
    }
}

#[cfg(all(test, feature = "rl"))]
mod tests {
    use super::*;

    #[test]
    fn test_visualization_creation() {
        let result = FlappyViz::new();
        assert!(result.is_ok(), "Should be able to create visualization");
    }
} 
