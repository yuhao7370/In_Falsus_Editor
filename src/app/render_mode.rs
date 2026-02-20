use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppViewMode {
    Editor2D,
    Render3D,
}

impl AppViewMode {
    pub fn label(self) -> &'static str {
        match self {
            AppViewMode::Editor2D => "2D editor",
            AppViewMode::Render3D => "3D render",
        }
    }
}

pub struct RenderModePresenter {
    mode: AppViewMode,
    render_target: RenderTarget,
    target_size: (u32, u32),
    orbit_phase: f32,
}

impl RenderModePresenter {
    pub fn new() -> Self {
        let target_size = Self::screen_size_px();
        let render_target = render_target(target_size.0, target_size.1);
        render_target.texture.set_filter(FilterMode::Linear);
        Self {
            mode: AppViewMode::Editor2D,
            render_target,
            target_size,
            orbit_phase: 0.0,
        }
    }

    pub fn is_render_3d(&self) -> bool {
        self.mode == AppViewMode::Render3D
    }

    pub fn toggle_mode(&mut self) -> AppViewMode {
        self.mode = match self.mode {
            AppViewMode::Editor2D => AppViewMode::Render3D,
            AppViewMode::Render3D => AppViewMode::Editor2D,
        };
        self.mode
    }

    pub fn begin_frame_capture(&mut self) {
        self.ensure_target_size();
        let mut cam = Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            self.target_size.0 as f32,
            self.target_size.1 as f32,
        ));
        cam.render_target = Some(self.render_target.clone());
        set_camera(&cam);
    }

    pub fn present(&mut self) {
        match self.mode {
            AppViewMode::Editor2D => self.present_2d(),
            AppViewMode::Render3D => self.present_3d(),
        }
    }

    fn ensure_target_size(&mut self) {
        let target_size = Self::screen_size_px();
        if self.target_size == target_size {
            return;
        }
        self.target_size = target_size;
        self.render_target = render_target(target_size.0, target_size.1);
        self.render_target.texture.set_filter(FilterMode::Linear);
    }

    fn screen_size_px() -> (u32, u32) {
        (
            screen_width().round().max(1.0) as u32,
            screen_height().round().max(1.0) as u32,
        )
    }

    fn present_2d(&self) {
        set_default_camera();
        clear_background(BLACK);
        draw_texture_ex(
            &self.render_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                flip_y: true,
                ..Default::default()
            },
        );
    }

    fn present_3d(&mut self) {
        set_default_camera();
        clear_background(Color::from_rgba(7, 10, 18, 255));

        self.orbit_phase = (self.orbit_phase + get_frame_time() * 0.35) % std::f32::consts::TAU;
        let radius = 10.0;
        let camera = Camera3D {
            position: vec3(
                radius * self.orbit_phase.cos(),
                3.5 + (self.orbit_phase * 1.4).sin() * 0.6,
                radius * self.orbit_phase.sin(),
            ),
            target: vec3(0.0, 1.4, 0.0),
            up: vec3(0.0, 1.0, 0.0),
            fovy: 36.0f32.to_radians(),
            ..Default::default()
        };
        set_camera(&camera);

        draw_grid(
            32,
            1.0,
            Color::from_rgba(80, 110, 160, 255),
            Color::from_rgba(38, 48, 76, 255),
        );

        let aspect = (self.target_size.0 as f32 / self.target_size.1.max(1) as f32).max(0.2);
        let half_h = 2.6;
        let half_w = half_h * aspect;
        let center = vec3(0.0, 1.4, 0.0);
        let vertices = vec![
            Vertex::new2(
                vec3(center.x - half_w, center.y + half_h, center.z),
                vec2(0.0, 1.0),
                WHITE,
            ),
            Vertex::new2(
                vec3(center.x - half_w, center.y - half_h, center.z),
                vec2(0.0, 0.0),
                WHITE,
            ),
            Vertex::new2(
                vec3(center.x + half_w, center.y - half_h, center.z),
                vec2(1.0, 0.0),
                WHITE,
            ),
            Vertex::new2(
                vec3(center.x + half_w, center.y + half_h, center.z),
                vec2(1.0, 1.0),
                WHITE,
            ),
        ];
        let mesh = Mesh {
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: Some(self.render_target.texture.clone()),
        };
        draw_mesh(&mesh);
        draw_cube_wires(
            center,
            vec3(half_w * 2.0, half_h * 2.0, 0.08),
            Color::from_rgba(150, 190, 240, 255),
        );
        draw_cube(
            vec3(0.0, -0.55, 0.0),
            vec3(2.2, 0.25, 2.2),
            None,
            Color::from_rgba(40, 58, 92, 255),
        );

        set_default_camera();
        draw_text(
            "3D render mode  |  P: back to 2D editor",
            16.0,
            30.0,
            28.0,
            Color::from_rgba(230, 236, 252, 255),
        );
    }
}
