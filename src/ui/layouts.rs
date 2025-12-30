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

pub fn create_two_column_layout(area: Rect, left_percentage: u16) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_percentage),
            Constraint::Percentage(100 - left_percentage),
        ])
        .split(area);
    
    (chunks[0], chunks[1])
}

pub fn create_two_row_layout(area: Rect, top_percentage: u16) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(top_percentage),
            Constraint::Percentage(100 - top_percentage),
        ])
        .split(area);
    
    (chunks[0], chunks[1])
}

pub fn create_grid_layout(area: Rect, rows: u16, cols: u16) -> Vec<Vec<Rect>> {
    let row_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Ratio(1, rows as u32))
        .collect();
    
    let row_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);
    
    row_chunks
        .iter()
        .map(|&row_area| {
            let col_constraints: Vec<Constraint> = (0..cols)
                .map(|_| Constraint::Ratio(1, cols as u32))
                .collect();
            
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints(col_constraints)
                .split(row_area)
        })
        .map(|row_chunks| row_chunks.to_vec())
        .collect()
}

pub fn create_adaptive_grid(area: Rect, item_count: usize) -> Vec<Rect> {
    if item_count == 0 {
        return vec![];
    }
    
    let (rows, cols) = calculate_grid_dimensions(item_count, area.width, area.height);
    
    let row_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Ratio(1, rows as u32))
        .collect();
    
    let row_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);
    
    let mut cells = Vec::new();
    let mut item_index = 0;
    
    for row_area in &*row_chunks {
        if item_index >= item_count {
            break;
        }
        
        let items_in_row = (item_count - item_index).min(cols);
        let col_constraints: Vec<Constraint> = (0..items_in_row)
            .map(|_| Constraint::Ratio(1, cols as u32))
            .collect();
        
        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(*row_area);
        
        for &cell in &col_chunks[..items_in_row] {
            cells.push(cell);
            item_index += 1;
            if item_index >= item_count {
                break;
            }
        }
    }
    
    cells
}

fn calculate_grid_dimensions(item_count: usize, width: u16, height: u16) -> (usize, usize) {
    if item_count <= 1 {
        return (1, 1);
    }
    
    let _aspect_ratio = width as f64 / height as f64;
    let target_ratio = 2.0;
    
    let _sqrt_count = (item_count as f64).sqrt();
    let mut best_rows = 1;
    let mut best_cols = item_count;
    let mut best_waste = item_count;
    
    for rows in 1..=item_count {
        let cols = (item_count + rows - 1) / rows;
        let total_cells = rows * cols;
        let waste = total_cells - item_count;
        
        let cell_ratio = (width as f64 / cols as f64) / (height as f64 / rows as f64);
        let ratio_diff = (cell_ratio - target_ratio).abs();
        
        if waste <= best_waste && ratio_diff < 1.0 {
            best_rows = rows;
            best_cols = cols;
            best_waste = waste;
        }
    }
    
    (best_rows, best_cols)
}

pub fn create_summary_layout(area: Rect, sections: usize) -> Vec<Rect> {
    if sections == 0 {
        return vec![];
    }
    
    let constraints: Vec<Constraint> = (0..sections)
        .map(|_| Constraint::Ratio(1, sections as u32))
        .collect();
        
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

#[allow(dead_code)]
pub struct SidebarLayout {
    pub sidebar: Rect,
    pub main: Rect,
}

#[allow(dead_code)]
pub fn create_sidebar_layout(area: Rect, sidebar_width: u16, left_sidebar: bool) -> SidebarLayout {
    let chunks = if left_sidebar {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(sidebar_width),
                Constraint::Min(0),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(sidebar_width),
            ])
            .split(area)
    };
    
    if left_sidebar {
        SidebarLayout {
            sidebar: chunks[0],
            main: chunks[1],
        }
    } else {
        SidebarLayout {
            sidebar: chunks[1],
            main: chunks[0],
        }
    }
}

#[allow(dead_code)]
pub struct ResponsiveLayout {
    pub is_compact: bool,
    pub areas: Vec<Rect>,
}

#[allow(dead_code)]
pub fn create_responsive_layout(area: Rect, min_width: u16, min_height: u16) -> ResponsiveLayout {
    let is_compact = area.width < min_width || area.height < min_height;
    
    if is_compact {
        let constraints = vec![
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
        ];
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);
        
        ResponsiveLayout {
            is_compact: true,
            areas: chunks.to_vec(),
        }
    } else {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Length(4),  // Summary
                Constraint::Min(0),     // Main content
                Constraint::Length(1),  // Footer
            ])
            .split(area);
        
        ResponsiveLayout {
            is_compact: false,
            areas: main_chunks.to_vec(),    
        }
    }
}

#[allow(dead_code)]
pub fn create_tabbed_layout(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Tab content
        ])
        .split(area);
    
    (chunks[0], chunks[1])
}

#[allow(dead_code)]
pub fn create_status_layout(area: Rect, status_items: usize) -> Vec<Rect> {
    if status_items == 0 {
        return vec![area];
    }
    
    let constraints: Vec<Constraint> = (0..status_items)
        .map(|_| Constraint::Ratio(1, status_items as u32))
        .collect(); 
    
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

pub mod utils {
    use super::*;
    
    #[allow(dead_code)]
    pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(area);
        
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
    
    #[allow(dead_code)]
    pub fn min_area_for_text(text: &str, margin: u16) -> (u16, u16) {
        let lines: Vec<&str> = text.lines().collect();
        let max_line_width = lines.iter().map(|line| line.len()).max().unwrap_or(0) as u16;
        let height = lines.len() as u16;
        
        (max_line_width + margin * 2, height + margin * 2)
    }
    
    #[allow(dead_code)]
    pub fn is_area_too_small(area: Rect, min_width: u16, min_height: u16) -> bool {
        area.width < min_width || area.height < min_height
    }
    
    #[allow(dead_code)]
    pub fn split_evenly(area: Rect, parts: usize, direction: Direction, spacing: u16) -> Vec<Rect> {
        if parts == 0 {
            return vec![];
        }
        
        let total_spacing = spacing * (parts.saturating_sub(1)) as u16;
        let available = match direction {
            Direction::Horizontal => area.width.saturating_sub(total_spacing),
            Direction::Vertical => area.height.saturating_sub(total_spacing),
        };
        
        let part_size = available / parts as u16;
        let mut constraints = Vec::new();
        
        for i in 0..parts {
            constraints.push(Constraint::Length(part_size));
            if i < parts - 1 && spacing > 0 {
                constraints.push(Constraint::Length(spacing));
            }
        }
        
        let chunks = Layout::default()
            .direction(direction)
            .constraints(constraints)
            .split(area);
        
        chunks.into_iter()
            .enumerate()
            .filter(|(i, _)| spacing == 0 || i % 2 == 0)
            .map(|(_, rect)| *rect)
            .collect()
    }
    
    #[allow(dead_code)]
    pub fn add_margin(area: Rect, margin: u16) -> Rect {
        Rect {
            x: area.x + margin,
            y: area.y + margin,
            width: area.width.saturating_sub(margin * 2),
            height: area.height.saturating_sub(margin * 2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_layout() {
        let area = Rect::new(0, 0, 80, 24);
        let layout = create_main_layout(area);
        
        assert_eq!(layout.tab_area.height, 3);
        assert_eq!(layout.summary_area.height, 4);
        assert_eq!(layout.footer_area.height, 1);
        assert!(layout.content_area.height > 0);
    }
    
    #[test]
    fn test_two_column_layout() {
        let area = Rect::new(0, 0, 80, 24);
        let (left, right) = create_two_column_layout(area, 30);
        
        assert!(left.width < right.width);
        assert_eq!(left.height, right.height);
    }
    
    #[test]
    fn test_grid_dimensions() {
        let result = calculate_grid_dimensions(4, 80, 24);
        assert!(result.0 * result.1 >= 4); 
        
        let result = calculate_grid_dimensions(6, 80, 24);
        assert!(result.0 * result.1 >= 6);
        assert_eq!(calculate_grid_dimensions(1, 80, 24), (1, 1));
    }
    
    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = utils::centered_rect(50, 50, area);
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 25);
        assert_eq!(centered.x, 25);
        assert!(centered.y >= 12 && centered.y <= 13);
    }
    
    #[test]
    fn test_min_area_for_text() {
        let text = "Hello\nWorld";
        let (width, height) = utils::min_area_for_text(text, 2);
        
        assert_eq!(width, 9); 
        assert_eq!(height, 6);
    }
}