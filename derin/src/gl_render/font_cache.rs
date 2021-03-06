// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::theme::ThemeFace;
use glyphydog::{Face, FTLib, Error};

use std::path::PathBuf;
use std::any::Any;
use std::rc::Rc;

enum FaceCached {
    Path {
        path: PathBuf,
        face_index: i32,
        face: Face<()>
    },
    Buffer {
        face_index: i32,
        face: Face<Rc<[u8]>>
    }
}

pub struct FontCache {
    lib: FTLib,
    faces: Vec<FaceCached>,
    max_faces: usize
}

impl FontCache {
    pub fn new() -> FontCache {
        FontCache {
            lib: FTLib::new(),
            faces: Vec::new(),
            max_faces: 16
        }
    }

    pub fn face(&mut self, theme_face: ThemeFace) -> Result<&mut Face<Any>, Error> {
        let mut cached_face_index = None;

        for (i, face) in self.faces.iter().enumerate() {
            let same_font = match (face, &theme_face) {
                (&FaceCached::Path{ref path, face_index, ..}, &ThemeFace::Path(ref b)) => face_index == b.face_index() && path == b.font_path(),
                (&FaceCached::Buffer{ref face, face_index, ..}, &ThemeFace::Buffer(ref b)) => face_index == b.face_index() && face.buffer() == b.font_buffer(),
                _ => false
            };
            if same_font {
                cached_face_index = Some(i);
            }
        }

        match cached_face_index {
            Some(i) => {
                if i >= 1 {
                    // Move the newest face to the front of the face list.
                    self.faces[..i+1].rotate_right(1);
                }
                Ok(match self.faces[0] {
                    FaceCached::Path{ref mut face, ..} => face,
                    FaceCached::Buffer{ref mut face, ..} => face
                })
            },
            None => {
                self.faces.insert(
                    0,
                    match theme_face {
                        ThemeFace::Path(path) => FaceCached::Path {
                            path: path.font_path().to_owned(),
                            face_index: path.face_index(),
                            face: Face::new_path(path.font_path(), path.face_index(), &self.lib)?
                        },
                        ThemeFace::Buffer(buffer) => FaceCached::Buffer {
                            face_index: buffer.face_index(),
                            face: Face::new(buffer.font_buffer().clone(), buffer.face_index(), &self.lib)?
                        },
                    }
                );
                if self.faces.len() > self.max_faces {
                    self.faces.pop();
                }
                Ok(match self.faces[0] {
                    FaceCached::Path{ref mut face, ..} => face,
                    FaceCached::Buffer{ref mut face, ..} => face
                })
            }
        }
    }
}
