use super::style::UiStyle;
use engine_render::font::TextPainter;
use engine_render::shape::ShapePainter;

pub struct UiContext<'a> {
    pub(crate) text_painter: &'a mut TextPainter,
    pub(crate) shape_painter: &'a mut ShapePainter,
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) style: UiStyle,
    pub(crate) cursor: [f32; 2],
    pub(crate) mouse_pos: [f32; 2],
    pub(crate) mouse_down: bool,
    pub(crate) mouse_clicked: bool,
    pub(crate) _focused_id: Option<u64>,
    pub(crate) next_id: u64,
}

impl<'a> UiContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        text_painter: &'a mut TextPainter,
        shape_painter: &'a mut ShapePainter,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        style: UiStyle,
        mouse_pos: [f32; 2],
        mouse_down: bool,
        mouse_clicked: bool,
    ) -> Self {
        Self {
            text_painter,
            shape_painter,
            device,
            queue,
            style,
            cursor: [0.0, 0.0],
            mouse_pos,
            mouse_down,
            mouse_clicked,
            _focused_id: None,
            next_id: 0,
        }
    }

    pub(crate) fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub(crate) fn mouse_in_rect(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let mx = self.mouse_pos[0];
        let my = self.mouse_pos[1];
        mx >= x && mx <= x + w && my >= y && my <= y + h
    }

    pub fn set_cursor(&mut self, pos: [f32; 2]) {
        self.cursor = pos;
    }

    pub fn advance(&mut self, dy: f32) {
        self.cursor[1] += dy + self.style.spacing;
    }

    pub fn style(&self) -> &UiStyle {
        &self.style
    }
    pub fn style_mut(&mut self) -> &mut UiStyle {
        &mut self.style
    }
    pub fn text_painter(&mut self) -> &mut TextPainter {
        self.text_painter
    }
    pub fn shape_painter(&mut self) -> &mut ShapePainter {
        self.shape_painter
    }
    pub(crate) fn estimate_text_width(&self, text: &str) -> f32 {
        text.len() as f32 * self.style.font_size * 0.6
    }
}
