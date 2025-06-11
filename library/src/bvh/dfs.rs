use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn depth_first_search<T, F, G>(
    root: Rc<RefCell<T>>,
    mut get_children: G,
    mut visit: F,
) where
    F: FnMut(&mut T, Option<Rc<RefCell<T>>>),
    G: FnMut(&T) -> (Option<Rc<RefCell<T>>>, Option<Rc<RefCell<T>>>),
{
    let mut stack = vec![(root, None)];

    while let Some((current, next_right)) = stack.pop() {
        let (left, right) = {
            let current_borrowed = current.borrow();
            get_children(&*current_borrowed)
        };
        
        visit(&mut *current.borrow_mut(), next_right.clone());
        
        if let Some(right_child) = right.clone() {
            stack.push((right_child, next_right.clone()));
        }

        if let Some(left_child) = left {
            stack.push((left_child, right.clone()));
        }
    }
}