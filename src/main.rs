extern crate sdl2;
extern crate rustfft;
extern crate num;
use num::complex::Complex;
use std::f64::consts::PI;
use std::f64;

const DIMS: (u32, u32) = (400, 400);
struct MouseInfo {
    pos: (i32, i32),
    is_down: bool,
}
struct DrawPad {
    // this is a grayscale buffer
    buffer: Vec<u8>,
}


fn write_drawpad_and_texture(grayscale: &mut DrawPad, color: &mut [u8], n: u8, pos: (i32, i32)) {
    // draw on internal buffer
    grayscale.buffer[pos.0 as usize + pos.1 as usize * DIMS.0 as usize] = n;
    // draw on screen
    let offset: usize = (pos.0 as usize) * 3 + (pos.1 as usize) * (DIMS.0 as usize* 3); 
    color[offset] = n;
    color[offset + 1] = n;
    color[offset + 2] = n;
}

fn radians_to_rgb(rad: f64) -> (f64, f64, f64) {
    //outputs values bounded from 0 to 1
    (
        ((rad - PI/2.0).sin()).powf(2.0), 
        ((rad - (4.0*PI/3.0) - PI/2.0).sin()).powf(2.0), 
        ((rad - (2.0*PI/3.0) - PI/2.0).sin()).powf(2.0)
    )
}


fn main() {
    // set up all the SDL2 nonsense
    let ctx = sdl2::init()
        .expect("Couldn't initalize SDL2.");
    let video_ctx = ctx.video()
        .expect("Couldn't initalize SDL2 video context.");
    let windows = (
        video_ctx.window("FFT Drawpad", DIMS.0, DIMS.1)
            .position_centered()
            .build()
            .expect("Couldn't build SDL2 window."),
        video_ctx.window("FFT Output", DIMS.0, DIMS.1)
            .position_centered()
            .build()
            .expect("Couldn't build SDL2 window."),
        );
    let mut canvases = (
        windows.0
            .into_canvas()
            .build()
            .expect("Couldn't build SDL2 window canvas."),
        windows.1
            .into_canvas()
            .build()
            .expect("Couldn't build SDL2 window canvas.")
    );
    let canvas_tex_creators = (
        canvases.0
            .texture_creator(),
        canvases.1
            .texture_creator()
    );
    let mut streaming_texs = (
        canvas_tex_creators.0
            .create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, DIMS.0, DIMS.1)
            .expect("Couldn't capture canvas for a streaming texture."),
        canvas_tex_creators.1
            .create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, DIMS.0, DIMS.1)
            .expect("Couldn't capture canvas for a streaming texture.")
    );
        
     
    // the main loop
    let mut mouse = MouseInfo {
        pos: (0, 0),
        is_down: false,
    };
    let mut need_to_fft = false;
    let mut draw_pad = DrawPad {
        buffer: vec![0; DIMS.0 as usize * DIMS.1 as usize],
    };
    // set up the FFT stuff
    let mut fft_in_buffer: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); DIMS.0 as usize * DIMS.1 as usize];
    let mut fft_out_buffer: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); DIMS.0 as usize * DIMS.1 as usize];
    let mut plan = rustfft::FFTplanner::new(false);
    let fft = plan.plan_fft(400 * 400);
    
    'main : loop {
        // take input
        for event in ctx.event_pump().expect("What the fuck?").poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::MouseButtonDown{x, y, mouse_btn: sdl2::mouse::MouseButton::Left, ..} => {
                    mouse.pos = (x, y);
                    mouse.is_down = true;
                }
                Event::MouseButtonUp{x, y, mouse_btn: sdl2::mouse::MouseButton::Left, ..} => {
                    mouse.pos = (x, y);
                    mouse.is_down = false;
                    need_to_fft = true;
                }
                Event::MouseMotion{x, y, ..} => {
                    // TODO: Event::MouseMotion::mousestate
                    mouse.pos = (x, y);
                }
                Event::Quit{..} => break 'main,
                _ => (),
            }
        }
        // UPDATE WINDOW 0: THE DRAWPAD
        // update texture
        streaming_texs.0.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
            if mouse.is_down {
                write_drawpad_and_texture(&mut draw_pad, buffer, 255, mouse.pos);
            }
        }).expect("what the fuck?");
        // PERFORM THE FFT ON WINDOW 0'S CONTENTS
        // first, transform the data into something that the dft crate can understand
        // that result is in fft_buffer -- which is already allocated
        if need_to_fft {
            for (index, value) in (&draw_pad.buffer).into_iter().enumerate() {
                fft_in_buffer[index] = Complex::new((*value as u16).into(), 0.0);
            }
            // perform the DFT
            fft.process(&mut fft_in_buffer, &mut fft_out_buffer);
            need_to_fft = false;
            // find the max value in the FFT out buffer
            let mut max_value: f64 = 0.0;
            for value in &fft_out_buffer {
                if value.norm().round() > max_value {
                    max_value = value.norm().round();
                }
            }
            // copy the fft_out_buffer WINDOW 1's canvas streaming texture
            streaming_texs.1.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                for (index, value) in (&fft_out_buffer).into_iter().enumerate() {
                    let amplitude: f64 = value.norm().floor(); // 0 to max_value
                    let brightness: f64 = 256.0 * amplitude / max_value; // 0 to 255
                    let rgb: (f64, f64, f64) = radians_to_rgb(value.arg()); // all elements are 0 to 1
                    buffer[index * 3    ] = (brightness * rgb.0).floor() as u8;
                    buffer[index * 3 + 1] = (brightness * rgb.1).floor() as u8;
                    buffer[index * 3 + 2] = (brightness * rgb.2).floor() as u8;
                }
            }).expect("What the fuck?");
        }
        // update canvases from texture
        canvases.0.clear();
        canvases.0.copy(&streaming_texs.0, None, None).expect("What the fuck?");
        canvases.0.present();
        canvases.1.clear();
        canvases.1.copy(&streaming_texs.1, None, None).expect("What the fuck?");
        canvases.1.present();
    }
}
