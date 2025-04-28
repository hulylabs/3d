pub(crate) fn remove_with_reorder<T, F>(victim: &mut Vec<T>, mut should_remove: F)
where
    F: FnMut(&T) -> bool,
{
    let mut i = 0;
    while i < victim.len() {
        if should_remove(&victim[i]) {
            victim.swap_remove(i);
            // don't increment `i` here, because the swapped-in element must be checked too
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_none() {
        fn make_vector() -> Vec<i32> {
            vec![1, 2, 3, 4, 5,]
        }
        
        let mut victim = make_vector();
        
        remove_with_reorder(&mut victim, |_| false);
        
        assert_eq!(victim, make_vector());
    }

    #[test]
    fn test_remove_all() {
        let mut v = vec![1, 2, 3, 4, 5, 6, 7,];
        remove_with_reorder(&mut v, |_| true);
        assert!(v.is_empty());
    }

    #[test]
    fn test_remove_some() {
        let mut victim = vec![1, 2, 3, 4, 5, 6];
        
        remove_with_reorder(&mut victim, |x| x % 2 == 0);
        
        victim.sort();
        assert_eq!(victim, vec![1, 3, 5]);
    }

    #[test]
    fn test_remove_first_last() {
        let mut victim = vec![10, 20, 30, 40, 50];
        
        remove_with_reorder(&mut victim, |x| *x == 10 || *x == 50);
        
        victim.sort();
        assert_eq!(victim, vec![20, 30, 40]);
    }

    #[test]
    fn test_empty_vec() {
        let mut victim: Vec<i32> = vec![];
        
        remove_with_reorder(&mut victim, |_| true);
        
        assert!(victim.is_empty());
    }
}