use crate::chart::{Chart, ChartEvent, Ease, FlickType};
use macroquad::prelude::*;
use sasa::AudioClip;

include!("falling/constants.rs");
include!("falling/note_types.rs");
include!("falling/timeline.rs");
include!("falling/waveform.rs");
include!("falling/editor_state.rs");

include!("falling/editor_methods_init.rs");
include!("falling/editor_methods_draw.rs");
include!("falling/editor_methods_minimap_layout.rs");
include!("falling/editor_methods_minimap_draw.rs");
include!("falling/editor_methods_minimap_input.rs");
include!("falling/editor_methods_render_event_header.rs");
include!("falling/editor_methods_render_progress_spectrum.rs");
include!("falling/editor_methods_render_ground.rs");
include!("falling/editor_methods_render_air.rs");
include!("falling/editor_methods_render_hitbox.rs");
include!("falling/editor_methods_render_place_cursor.rs");
include!("falling/editor_methods_render_skyarea_shape.rs");
include!("falling/editor_methods_input_seek_ground.rs");
include!("falling/editor_methods_input_air_select.rs");
include!("falling/editor_methods_input_hover_overlap.rs");
include!("falling/editor_methods_input_collect.rs");
include!("falling/editor_methods_input_drag.rs");
include!("falling/editor_methods_internal.rs");

include!("falling/hit_math.rs");
include!("falling/debug_draw.rs");
include!("falling/note_style.rs");
include!("falling/ui_helpers.rs");
include!("falling/chart_extract.rs");
