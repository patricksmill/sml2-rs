//! Optional software previews for reviewing layout without a window server.
//! These are UI review artifacts, not GPU or gameplay parity assertions.

use super::workspace_ui::frame;
use crate::app::{screenshot, tests::app};
use eframe::egui::{self, Color32, ColorImage, TextureId};
use std::collections::HashMap;

#[test]
#[ignore = "exports UI review PNGs into the system temporary directory"]
fn export_workspace_previews() {
    let directory = std::env::temp_dir().join("sml2-editor-previews");
    std::fs::create_dir_all(&directory).unwrap();
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let mut textures: HashMap<TextureId, ColorImage> = HashMap::new();
    for (width, height) in [(480, 400), (800, 720), (1600, 1000)] {
        for pass in 0..3 {
            let output = frame(&mut app, &ctx, (width as f32, height as f32), vec![]);
            for (id, delta) in &output.textures_delta.set {
                let egui::ImageData::Color(image) = &delta.image;
                if let Some([x, y]) = delta.pos {
                    let texture = textures.get_mut(id).unwrap();
                    for row in 0..image.size[1] {
                        let at = (y + row) * texture.size[0] + x;
                        let source = row * image.size[0];
                        texture.pixels[at..at + image.size[0]]
                            .copy_from_slice(&image.pixels[source..source + image.size[0]]);
                    }
                } else {
                    textures.insert(*id, (**image).clone());
                }
            }
            if pass == 2 {
                let mut image = ColorImage::filled([width, height], Color32::BLACK);
                let meshes = ctx.tessellate(output.shapes, output.pixels_per_point);
                for primitive in meshes {
                    let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive else {
                        continue;
                    };
                    let texture = &textures[&mesh.texture_id];
                    for triangle in mesh.indices.chunks_exact(3) {
                        let v: [_; 3] =
                            std::array::from_fn(|i| mesh.vertices[triangle[i] as usize]);
                        let cross = |a: egui::Vec2, b: egui::Vec2| a.x * b.y - a.y * b.x;
                        let area = cross(v[1].pos - v[0].pos, v[2].pos - v[0].pos);
                        if area.abs() < 0.001 {
                            continue;
                        }
                        let bounds = egui::Rect::from_points(&v.map(|v| v.pos))
                            .intersect(primitive.clip_rect)
                            .intersect(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width as f32, height as f32),
                            ));
                        for y in bounds.top().ceil() as usize..bounds.bottom().ceil() as usize {
                            for x in bounds.left().ceil() as usize..bounds.right().ceil() as usize {
                                let p = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
                                let weights = [
                                    cross(v[1].pos - p, v[2].pos - p) / area,
                                    cross(v[2].pos - p, v[0].pos - p) / area,
                                    cross(v[0].pos - p, v[1].pos - p) / area,
                                ];
                                if weights.iter().any(|&w| w < 0.0) {
                                    continue;
                                }
                                let uv = v[0].uv.to_vec2() * weights[0]
                                    + v[1].uv.to_vec2() * weights[1]
                                    + v[2].uv.to_vec2() * weights[2];
                                let tx = ((uv.x * texture.size[0] as f32) as usize)
                                    .min(texture.size[0] - 1);
                                let ty = ((uv.y * texture.size[1] as f32) as usize)
                                    .min(texture.size[1] - 1);
                                let texel = texture.pixels[ty * texture.size[0] + tx].to_array();
                                let colors = v.map(|v| v.color.to_array());
                                let rgba: [f32; 4] = std::array::from_fn(|c| {
                                    (0..3)
                                        .map(|i| f32::from(colors[i][c]) * weights[i])
                                        .sum::<f32>()
                                        * f32::from(texel[c])
                                        / 255.0
                                });
                                let at = y * width + x;
                                let previous = image.pixels[at].to_array();
                                let rgb: [u8; 3] = std::array::from_fn(|c| {
                                    (rgba[c] + f32::from(previous[c]) * (1.0 - rgba[3] / 255.0))
                                        .clamp(0.0, 255.0) as u8
                                });
                                image.pixels[at] = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            }
                        }
                    }
                }
                let file = directory.join(format!("workspace-{width}x{height}.png"));
                screenshot::save(&file, &image).unwrap();
                eprintln!("UI preview: {}", file.display());
            }
            for id in output.textures_delta.free {
                textures.remove(&id);
            }
        }
    }
    std::fs::remove_dir_all(path).ok();
}
