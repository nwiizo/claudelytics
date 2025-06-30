use std::cmp::Ordering;

/// Generic comparison trait for sortable types
pub trait Comparable {
    /// Compare two instances by a specific criterion
    fn compare(&self, other: &Self) -> Ordering;
}

/// Helper for safe float comparison with NaN handling
pub fn compare_floats(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// Calculate efficiency (tokens per dollar) safely
pub fn calculate_efficiency(tokens: u64, cost: f64) -> f64 {
    if cost > 0.0 {
        tokens as f64 / cost
    } else {
        0.0
    }
}

/// Generic function to get the last N items from a slice
pub fn get_last_n_items<T: Clone>(items: &[T], n: usize) -> Vec<T> {
    items.iter().rev().take(n).cloned().collect()
}

/// Calculate average from a slice of numeric values
pub fn calculate_average<T>(values: &[T]) -> f64
where
    T: Into<f64> + Copy,
{
    if values.is_empty() {
        return 0.0;
    }
    
    let sum: f64 = values.iter().map(|&v| v.into()).sum();
    sum / values.len() as f64
}

/// Generic sorting with custom comparator and order
pub fn sort_with_order<T, F>(items: &mut [T], compare_fn: F, ascending: bool)
where
    F: Fn(&T, &T) -> Ordering,
{
    items.sort_by(|a, b| {
        let cmp = compare_fn(a, b);
        if ascending {
            cmp
        } else {
            cmp.reverse()
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_floats() {
        assert_eq!(compare_floats(1.0, 2.0), Ordering::Less);
        assert_eq!(compare_floats(2.0, 1.0), Ordering::Greater);
        assert_eq!(compare_floats(1.0, 1.0), Ordering::Equal);
        assert_eq!(compare_floats(f64::NAN, 1.0), Ordering::Equal);
    }

    #[test]
    fn test_calculate_efficiency() {
        assert_eq!(calculate_efficiency(1000, 10.0), 100.0);
        assert_eq!(calculate_efficiency(1000, 0.0), 0.0);
        assert_eq!(calculate_efficiency(0, 10.0), 0.0);
    }

    #[test]
    fn test_get_last_n_items() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(get_last_n_items(&items, 3), vec![5, 4, 3]);
        assert_eq!(get_last_n_items(&items, 10), vec![5, 4, 3, 2, 1]);
        assert_eq!(get_last_n_items(&items, 0), Vec::<i32>::new());
    }

    #[test]
    fn test_calculate_average() {
        assert_eq!(calculate_average(&[1, 2, 3, 4, 5]), 3.0);
        assert_eq!(calculate_average(&[10.0, 20.0]), 15.0);
        assert_eq!(calculate_average(&[]), 0.0);
    }
}