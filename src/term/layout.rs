#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    pub fn right(self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constraint {
    Length(u16),
    Min(u16),
    Percentage(u16),
    Ratio(u32, u32),
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    direction: Direction,
    constraints: Vec<Constraint>,
    margin: u16,
}

impl Layout {
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn constraints<C: Into<Vec<Constraint>>>(mut self, constraints: C) -> Self {
        self.constraints = constraints.into();
        self
    }

    pub fn margin(mut self, margin: u16) -> Self {
        self.margin = margin;
        self
    }

    pub fn split(&self, area: Rect) -> Vec<Rect> {
        let inner = if self.margin > 0 {
            let m2 = self.margin.saturating_mul(2);
            Rect {
                x: area.x.saturating_add(self.margin),
                y: area.y.saturating_add(self.margin),
                width: area.width.saturating_sub(m2),
                height: area.height.saturating_sub(m2),
            }
        } else {
            area
        };

        let total = match self.direction {
            Direction::Vertical => inner.height,
            Direction::Horizontal => inner.width,
        };

        let sizes = solve_constraints(&self.constraints, total);
        let mut rects = Vec::with_capacity(sizes.len());
        let mut offset = 0u16;
        for sz in sizes {
            let rect = match self.direction {
                Direction::Vertical => Rect::new(inner.x, inner.y.saturating_add(offset), inner.width, sz),
                Direction::Horizontal => Rect::new(inner.x.saturating_add(offset), inner.y, sz, inner.height),
            };
            offset = offset.saturating_add(sz);
            rects.push(rect);
        }
        rects
    }
}

pub fn solve_constraints(constraints: &[Constraint], total: u16) -> Vec<u16> {
    let n = constraints.len();
    if n == 0 {
        return Vec::new();
    }
    if total == 0 {
        return vec![0; n];
    }

    let mut sizes = vec![0u16; n];
    let mut flexible = vec![false; n];
    let mut used: u32 = 0;

    // Pass 1: Fixed Length, Percentage, Ratio, and Min baseline
    for (i, c) in constraints.iter().enumerate() {
        let sz = match *c {
            Constraint::Length(l) => l,
            Constraint::Percentage(p) => ((total as u32 * p.min(100) as u32) / 100) as u16,
            Constraint::Ratio(num, den) => {
                if den == 0 {
                    0
                } else {
                    ((total as u64 * num as u64) / den as u64).min(total as u64) as u16
                }
            }
            Constraint::Min(m) => {
                flexible[i] = true;
                m
            }
        };
        sizes[i] = sz;
        used += sz as u32;
    }

    if used < total as u32 {
        let mut remaining = total as u32 - used;
        let flex_indices: Vec<usize> = (0..n).filter(|&i| flexible[i]).collect();
        if !flex_indices.is_empty() {
            let share = remaining / flex_indices.len() as u32;
            let mut rem = remaining % flex_indices.len() as u32;
            for idx in flex_indices {
                let extra = share + if rem > 0 { rem -= 1; 1 } else { 0 };
                sizes[idx] = sizes[idx].saturating_add(extra as u16);
            }
        } else {
            // If all constraints are Percentage/Ratio and sum < total due to integer rounding, give remainder to last item
            let has_prop = constraints
                .iter()
                .any(|c| matches!(c, Constraint::Percentage(_) | Constraint::Ratio(_, _)));
            if has_prop {
                if let Some(last) = sizes.last_mut() {
                    *last = last.saturating_add(remaining as u16);
                    remaining = 0;
                }
            }
            let _ = remaining;
        }
    } else if used > total as u32 {
        // Shrink proportionally / from end so sum never exceeds total
        let mut excess = used - total as u32;
        // Shrink Min items first, then Percentage/Ratio, then Length
        for pass in 0..3 {
            for i in (0..n).rev() {
                if excess == 0 {
                    break;
                }
                let matches_pass = match (pass, constraints[i]) {
                    (0, Constraint::Min(_)) => true,
                    (1, Constraint::Percentage(_) | Constraint::Ratio(_, _)) => true,
                    (2, Constraint::Length(_)) => true,
                    _ => false,
                };
                if matches_pass && sizes[i] > 0 {
                    let take = (sizes[i] as u32).min(excess);
                    sizes[i] -= take as u16;
                    excess -= take;
                }
            }
        }
    }

    sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_vertical_and_horizontal() {
        let area = Rect::new(0, 0, 100, 30);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], Rect::new(0, 0, 100, 3));
        assert_eq!(chunks[1], Rect::new(0, 3, 100, 26));
        assert_eq!(chunks[2], Rect::new(0, 29, 100, 1));

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)])
            .split(Rect::new(0, 0, 100, 10));
        assert_eq!(cols[0].width + cols[1].width + cols[2].width, 100);
    }

    #[test]
    fn test_layout_overflow_clamping() {
        let sizes = solve_constraints(&[Constraint::Length(50), Constraint::Length(50), Constraint::Min(10)], 60);
        assert_eq!(sizes.iter().sum::<u16>(), 60);
    }
}
