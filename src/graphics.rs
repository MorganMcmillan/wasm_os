#![allow(clippy::too_many_arguments)]

// Note: this driver is actually intended for all Wasm-os distributions, Provided they use 8-bit
// pixel graphics

mod camera;
mod color;
mod draw_region;
pub mod graphics_state;

use crate::{
    graphics::{color::Color, draw_region::DrawRegion, graphics_state::FONT_SIZE},
    kernel::{ProcessContext, ProcessLinker},
};

/// Loads all the system functions related to drawing pixels to an address.
pub fn load_graphics_functions<T>(linker: &mut ProcessLinker<T>) -> wasmtime::Result<()> {
    // State configuration

    linker.func_wrap(
        "env",
        "set_draw_region",
        |mut ctx: ProcessContext<T>, address: u32, width: u32, height: u32| {
            let address = address as usize;
            let draw_region = DrawRegion::new(width, height);
            ctx.data()
                .assert_memory_size(address, draw_region.area() as usize, "draw_region");

            ctx.data_mut().graphics_state.draw_address = address;
            ctx.data_mut().graphics_state.draw_region = draw_region;
        },
    )?;

    linker.func_wrap(
        "env",
        "clear_draw_region",
        |ctx: ProcessContext<T>, color: u32| {
            let address = ctx.data().get_draw_address();
            unsafe {
                address.write_bytes(
                    color as u8,
                    ctx.data().graphics_state.draw_region.area() as usize,
                );
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "set_transparency_color",
        |mut ctx: ProcessContext<T>, color: u32| -> u32 {
            let old_color = ctx.data_mut().graphics_state.transparency_color;
            ctx.data_mut().graphics_state.transparency_color = color as u8;
            old_color as u32
        },
    )?;

    linker.func_wrap(
        "env",
        "set_camera",
        |mut ctx: ProcessContext<T>, x: i32, y: i32| {
            ctx.data_mut().graphics_state.camera.set_position(x, y);
        },
    )?;

    linker.func_wrap("env", "get_camera_x", |mut ctx: ProcessContext<T>| -> i32 {
        ctx.data_mut().graphics_state.camera.x
    })?;

    linker.func_wrap("env", "get_camera_y", |mut ctx: ProcessContext<T>| -> i32 {
        ctx.data_mut().graphics_state.camera.y
    })?;

    linker.func_wrap(
        "env",
        "set_font",
        |mut ctx: ProcessContext<T>, font: i32| {
            ctx.data()
                .assert_memory_size(font as usize, FONT_SIZE, "font");
            ctx.data_mut().graphics_state.set_font(font as u32);
        },
    )?;

    linker.func_wrap("env", "use_default_font", |mut ctx: ProcessContext<T>| {
        ctx.data_mut().graphics_state.use_default_font();
    })?;

    linker.func_wrap(
        "env",
        "set_fill_pattern",
        |mut ctx: ProcessContext<T>, pattern: u64| {
            ctx.data_mut().graphics_state.set_fill_pattern(pattern);
        },
    )?;

    linker.func_wrap(
        "env",
        "get_fill_pattern",
        |mut ctx: ProcessContext<T>| -> u64 { ctx.data_mut().graphics_state.get_fill_pattern() },
    )?;

    linker.func_wrap(
        "env",
        "set_secondary_palette",
        |mut ctx: ProcessContext<T>, address: i32| {
            ctx.data_mut()
                .graphics_state
                .set_secondary_palette(address as u32);
        },
    )?;

    linker.func_wrap(
        "env",
        "set_flags",
        |mut ctx: ProcessContext<T>, flags: u32| {
            ctx.data_mut().graphics_state.set_flags(flags as u8);
        },
    )?;
    linker.func_wrap(
        "env",
        "unset_flags",
        |mut ctx: ProcessContext<T>, flags: u32| {
            ctx.data_mut().graphics_state.unset_flags(flags as u8);
        },
    )?;

    linker.func_wrap(
        "env",
        "set_color_mode",
        |mut ctx: ProcessContext<T>, mode: u32| {
            ctx.data_mut().graphics_state.set_color_mode(mode as u8);
        },
    )?;

    // Drawing

    linker.func_wrap(
        "env",
        "draw_pixel",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, pixel: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_pixel_checked(draw_address, x, y, pixel as Color);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_line",
        |mut ctx: ProcessContext<T>, x1: i32, y1: i32, x2: i32, y2: i32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_line(draw_address, x1, y1, x2, y2, color as Color);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_textured_line",
        |mut ctx: ProcessContext<T>,
         x1: i32,
         y1: i32,
         x2: i32,
         y2: i32,
         texture: i32,
         tex_width: u32,
         tex_height: u32,
         tex_x: f32,
         tex_y: f32,
         tex_dx: f32,
         tex_dy: f32| {
            let draw_address = ctx.data().get_draw_address();

            let texture = ctx
                .data()
                .get_memory(texture as usize, (tex_width * tex_height) as usize)
                .as_mut_ptr();

            ctx.data_mut().graphics_state.draw_textured_line(
                draw_address,
                x1,
                y1,
                x2,
                y2,
                texture,
                tex_width,
                tex_height,
                tex_x,
                tex_y,
                tex_dx,
                tex_dy,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_hline",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, width: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_hline(draw_address, x, y, width, color as Color);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_vline",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, height: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_vline(draw_address, x, y, height, color as Color);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_rectangle",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, width: u32, height: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_rectangle(
                draw_address,
                x,
                y,
                width,
                height,
                color as Color,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_filled_rectangle",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, width: u32, height: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_filled_rectangle(
                draw_address,
                x,
                y,
                width,
                height,
                color as Color,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_round_rectangle",
        |mut ctx: ProcessContext<T>,
         x: i32,
         y: i32,
         width: u32,
         height: u32,
         radius: u32,
         color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_round_rectangle(
                draw_address,
                x,
                y,
                width,
                height,
                radius,
                color as Color,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_filled_round_rectangle",
        |mut ctx: ProcessContext<T>,
         x: i32,
         y: i32,
         width: u32,
         height: u32,
         radius: u32,
         color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_filled_round_rectangle(
                draw_address,
                x,
                y,
                width,
                height,
                radius,
                color as Color,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_circle",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, radius: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut()
                .graphics_state
                .draw_circle(draw_address, x, y, radius, color as Color);
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_filled_circle",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, radius: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_filled_circle(
                draw_address,
                x,
                y,
                radius,
                color as Color,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_ellipse",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, x_radius: u32, y_radius: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_ellipse(
                draw_address,
                x,
                y,
                x_radius,
                y_radius,
                color as Color,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_filled_ellipse",
        |mut ctx: ProcessContext<T>, x: i32, y: i32, x_radius: u32, y_radius: u32, color: u32| {
            let draw_address = ctx.data().get_draw_address();
            ctx.data_mut().graphics_state.draw_filled_ellipse(
                draw_address,
                x,
                y,
                x_radius,
                y_radius,
                color as Color,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_sprite",
        |mut ctx: ProcessContext<T>,
         x: i32,
         y: i32,
         sprite: i32,
         spr_width: u32,
         spr_height: u32| {
            let draw_address = ctx.data().get_draw_address();

            let sprite = ctx
                .data()
                .get_memory(sprite as usize, (spr_width * spr_height) as usize)
                .as_mut_ptr();

            let memory = ctx.data().get_entire_memory().as_ptr();

            ctx.data_mut().graphics_state.draw_sprite(
                draw_address,
                memory,
                x,
                y,
                sprite,
                spr_width,
                spr_height,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_map",
        |mut ctx: ProcessContext<T>,
         map: i32,
         map_width: u32,
         map_height: u32,
         spritesheet: i32,
         spr_width: u32,
         spr_height: u32| {
            let draw_address = ctx.data().get_draw_address();

            let map_width = map_width as usize;
            let map_height = map_height as usize;

            let map = ctx
                .data()
                .get_memory(map as usize, map_width * map_height)
                .as_mut_ptr();

            let spritesheet = ctx
                .data()
                .get_memory(
                    spritesheet as usize,
                    (spr_width * spr_height * 256) as usize,
                )
                .as_mut_ptr();

            ctx.data_mut().graphics_state.draw_map(
                draw_address,
                map,
                map_width,
                map_height,
                spritesheet,
                spr_width,
                spr_height,
            );
        },
    )?;

    linker.func_wrap(
        "env",
        "draw_text",
        |mut ctx: ProcessContext<T>,
         text_ptr: i32,
         text_len: u32,
         x: i32,
         y: i32,
         fg: u32,
         bg: u32| {
            let draw_address = ctx.data().get_draw_address();

            let memory = ctx.data().get_entire_memory().as_ptr();
            let text = ctx.data().get_memory(text_ptr as usize, text_len as usize);

            ctx.data_mut().graphics_state.draw_text(
                draw_address,
                memory,
                text,
                x,
                y,
                fg as u8,
                bg as u8,
            );
        },
    )?;

    Ok(())
}
