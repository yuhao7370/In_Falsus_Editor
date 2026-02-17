fn lane_from_x(x: f32, lanes_x: f32, lane_w: f32) -> usize {
    ((x - lanes_x) / lane_w).floor().clamp(0.0, (LANE_COUNT as f32) - 1.0) as usize
}

fn adaptive_ui_scale() -> f32 {
    (screen_width() / REFERENCE_WIDTH)
        .min(screen_height() / REFERENCE_HEIGHT)
        .clamp(0.75, 3.5)
}

fn scaled_px(px: f32) -> f32 {
    px * adaptive_ui_scale()
}

fn scaled_font_size(base: f32, min: u16, max: u16) -> u16 {
    let size = (base * adaptive_ui_scale()).round();
    size.clamp(min as f32, max as f32) as u16
}

fn push_best_hit_candidate(candidates: &mut Vec<HitCandidate>, candidate: HitCandidate) {
    if let Some(existing) = candidates
        .iter_mut()
        .find(|item| {
            item.note_id == candidate.note_id
                && item.scope == candidate.scope
                && item.air_target == candidate.air_target
                && item.part == candidate.part
        })
    {
        if should_replace_hit_candidate(*existing, candidate) {
            *existing = candidate;
        }
    } else {
        candidates.push(candidate);
    }
}

fn should_replace_hit_candidate(current: HitCandidate, incoming: HitCandidate) -> bool {
    if (incoming.distance_sq - current.distance_sq).abs() > 0.01 {
        return incoming.distance_sq < current.distance_sq;
    }

    let current_rank = hit_part_rank(current.part);
    let incoming_rank = hit_part_rank(incoming.part);
    if incoming_rank != current_rank {
        return incoming_rank > current_rank;
    }
    incoming.z_order > current.z_order
}

fn sort_hit_candidates(candidates: &mut Vec<HitCandidate>) {
    candidates.sort_by(|a, b| {
        hit_part_rank(b.part)
            .cmp(&hit_part_rank(a.part))
            .then_with(|| a.distance_sq.total_cmp(&b.distance_sq))
            .then_with(|| b.z_order.cmp(&a.z_order))
            .then_with(|| a.note_id.cmp(&b.note_id))
    });
}

fn hit_part_rank(part: HitPart) -> u8 {
    match part {
        HitPart::Head | HitPart::Tail => 2,
        HitPart::Body => 1,
    }
}

fn distance_sq(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

fn distance_sq_to_rect(px: f32, py: f32, rect: Rect) -> f32 {
    let rx1 = rect.x;
    let ry1 = rect.y;
    let rx2 = rect.x + rect.w;
    let ry2 = rect.y + rect.h;
    let cx = px.clamp(rx1, rx2);
    let cy = py.clamp(ry1, ry2);
    distance_sq(px, py, cx, cy)
}

fn note_end_hit_rect(x: f32, w: f32, center_y: f32) -> Rect {
    let pad_x = scaled_px(NOTE_HEAD_HIT_PAD_X);
    let half_h = scaled_px(NOTE_HEAD_HIT_HALF_H);
    Rect::new(
        x - pad_x,
        center_y - half_h,
        (w + pad_x * 2.0).max(1.0),
        (half_h * 2.0).max(1.0),
    )
}

fn note_body_hit_rect(x: f32, w: f32, y1: f32, y2: f32) -> Rect {
    let edge_gap = scaled_px(NOTE_BODY_EDGE_GAP_Y);
    let pad_x = scaled_px(NOTE_BODY_HIT_PAD_X);
    let top_raw = y1.min(y2);
    let bottom_raw = y1.max(y2);
    let top = (top_raw + edge_gap).min(bottom_raw);
    let bottom = (bottom_raw - edge_gap).max(top);
    let body_w = (w + pad_x * 2.0).max(1.0);
    let thin_h = scaled_px(2.0);
    if bottom - top < thin_h {
        let center_y = (top_raw + bottom_raw) * 0.5;
        return Rect::new(x - pad_x, center_y - thin_h, body_w, thin_h * 2.0);
    }
    Rect::new(
        x - pad_x,
        top,
        body_w,
        (bottom - top).max(1.0),
    )
}

fn flick_rect_hitbox(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32, side_h: f32) -> Rect {
    // Rectangle hitbox aligned to the actual flick footprint.
    flick_shape_bounds(note, note_x, note_w, head_y, side_h)
}

fn skyarea_screen_x_range_at_progress(
    split_rect: Rect,
    shape: SkyAreaShape,
    p: f32,
) -> (f32, f32) {
    let p = p.clamp(0.0, 1.0);
    let left_norm = lerp(
        shape.start_left_norm,
        shape.end_left_norm,
        ease_progress(shape.left_ease, p),
    )
    .clamp(0.0, 1.0);
    let right_norm = lerp(
        shape.start_right_norm,
        shape.end_right_norm,
        ease_progress(shape.right_ease, p),
    )
    .clamp(0.0, 1.0);
    (
        split_rect.x + left_norm * split_rect.w,
        split_rect.x + right_norm * split_rect.w,
    )
}

fn skyarea_body_vertical_range(head_y: f32, tail_y: f32) -> (f32, f32) {
    let edge_gap = scaled_px(NOTE_BODY_EDGE_GAP_Y);
    let min_y = head_y.min(tail_y);
    let max_y = head_y.max(tail_y);
    if max_y - min_y <= edge_gap * 2.0 {
        let mid = (min_y + max_y) * 0.5;
        let thin_h = scaled_px(2.0);
        return (mid - thin_h, mid + thin_h);
    }
    (min_y + edge_gap, max_y - edge_gap)
}

fn skyarea_body_hit_distance_sq(
    mx: f32,
    my: f32,
    split_rect: Rect,
    shape: SkyAreaShape,
    head_y: f32,
    tail_y: f32,
) -> Option<f32> {
    let dy = tail_y - head_y;
    if dy.abs() <= 0.000_1 {
        return None;
    }

    let (body_top, body_bottom) = skyarea_body_vertical_range(head_y, tail_y);
    if my < body_top || my > body_bottom {
        return None;
    }

    let p = ((my - head_y) / dy).clamp(0.0, 1.0);
    let (left_x, right_x) = skyarea_screen_x_range_at_progress(split_rect, shape, p);
    let pad_x = scaled_px(NOTE_BODY_HIT_PAD_X);
    let x1 = left_x.min(right_x) - pad_x;
    let x2 = left_x.max(right_x) + pad_x;
    if mx < x1 || mx > x2 {
        return None;
    }

    let center_x = (x1 + x2) * 0.5;
    let dist = mx - center_x;
    Some(dist * dist)
}

fn quantize_overlap_anchor(x: f32, y: f32) -> (i32, i32) {
    let anchor_px = scaled_px(OVERLAP_CYCLE_ANCHOR_PX).max(1.0);
    (
        (x / anchor_px).round() as i32,
        (y / anchor_px).round() as i32,
    )
}

fn hit_signature_item(candidate: &HitCandidate) -> HitSignatureItem {
    HitSignatureItem {
        note_id: candidate.note_id,
        scope: candidate.scope,
        air_target: candidate.air_target,
        part: candidate.part,
    }
}

fn canonical_hit_signature(items: &[HitSignatureItem]) -> Vec<HitSignatureItem> {
    let mut signature = items.to_vec();
    signature.sort_by(|a, b| {
        hit_scope_rank(a.scope)
            .cmp(&hit_scope_rank(b.scope))
            .then_with(|| hit_part_rank(b.part).cmp(&hit_part_rank(a.part)))
            .then_with(|| air_target_rank(a.air_target).cmp(&air_target_rank(b.air_target)))
            .then_with(|| a.note_id.cmp(&b.note_id))
    });
    signature
}

fn hit_scope_rank(scope: HitScope) -> u8 {
    match scope {
        HitScope::Ground => 0,
        HitScope::Air => 1,
        HitScope::Mixed => 2,
    }
}

fn air_target_rank(target: AirDragTarget) -> u8 {
    match target {
        AirDragTarget::Body => 0,
        AirDragTarget::SkyHead => 1,
        AirDragTarget::SkyTail => 2,
    }
}


