use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
};

#[derive(Debug, Clone)]
pub struct MainLayout {
    pub tab_area: Rect,
    pub summary_area: Rect,
    pub content_area: Rect,
    pub footer_area: Rect,
}

pub fn create_main_layout(area: Rect) -> MainLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tab bar
            Constraint::Length(4),  // Summary bar
            Constraint::Min(0),     // Main content
            Constraint::Length(1),  // Footer
        ])
        .split(area);

    MainLayout {
        tab_area: chunks[0],
        summary_area: chunks[1],
        content_area: chunks[2],
        footer_area: chunks[3],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_layout() {
        let area = ratatui::layout::Rect::new(0, 0, 80, 24);
        let layout = create_main_layout(area);
        
        assert_eq!(layout.tab_area.height, 3);
        assert_eq!(layout.summary_area.height, 4);
        assert_eq!(layout.footer_area.height, 1);
        assert!(layout.content_area.height > 0);
    }
}
