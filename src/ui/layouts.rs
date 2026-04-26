#![allow(dead_code)]

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
    
    #[test]
    fn test_two_column_layout() {
        let area = ratatui::layout::Rect::new(0, 0, 80, 24);
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
}
