use crate::scene::sdf::sdf::Sdf;
use std::cmp::max;
use std::collections::HashSet;
use std::rc::Rc;

pub(super) fn depth_first_search<T, F>(root: Rc<dyn Sdf>, context: &mut T, mut visit: F)
where
    F: FnMut(Rc<dyn Sdf>, &mut T, usize),
{
    #[derive(Clone)]
    struct Node { data: Rc<dyn Sdf>, levels_below: usize, }
    
    let mut traversal_front: Vec<Node> = vec![Node{ data: root, levels_below: 0 }];
    let mut visited: HashSet<*const dyn Sdf> = HashSet::new();

    loop {
        if traversal_front.is_empty() {
            break;
        }

        let candidate = traversal_front[traversal_front.len() - 1].clone();

        if visited.insert(candidate.data.as_ref()) {
            for child in candidate.data.children().iter().rev() {
                traversal_front.push(Node{ data: child.clone(), levels_below: 0 });   
            }
        } else {
            traversal_front.pop();

            for ancestor in &mut traversal_front {
                if visited.contains(&Rc::as_ptr(&ancestor.data)) {
                    ancestor.levels_below = max(ancestor.levels_below, candidate.levels_below + 1);
                }
            }
            
            visit(candidate.data.clone(), context, candidate.levels_below);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::sdf::dummy_sdf::tests::DummySdf;
    use crate::scene::sdf::sdf_sphere::SdfSphere;
    use crate::scene::sdf::sdf_union::SdfUnion;

    #[test]
    fn test_zero_levels_below() {
        let levels_blow = evaluate_levels_below(Rc::new(DummySdf::default()));
        assert_eq!(levels_blow, 0);
    }

    #[test]
    fn test_single_levels_below() {
        let tree =
            SdfUnion::new(
                SdfSphere::new(1.0),
                SdfSphere::new(2.0),
            );
        let levels_blow = evaluate_levels_below(tree.clone());
        assert_eq!(levels_blow, 1);
    }

    #[test]
    fn test_asymmetric_tree_levels_below_root() {
        let tree = 
            SdfUnion::new(
                SdfUnion::new(
                    SdfUnion::new(
                        SdfSphere::new(1.0),
                        SdfSphere::new(2.0),
                    ),
                    SdfSphere::new(3.0),
                ),
                SdfUnion::new(
                    SdfSphere::new(4.0),
                    SdfSphere::new(5.0),
                ),
        );
        let levels_blow = evaluate_levels_below(tree.clone());
        assert_eq!(levels_blow, 3);
    }

    #[test]
    fn test_symmetric_tree_levels_below_root() {
        let tree =
            SdfUnion::new(
                SdfUnion::new(
                    SdfUnion::new(
                        SdfSphere::new(1.0),
                        SdfSphere::new(2.0),
                    ),
                    SdfSphere::new(3.0),
                ),
                SdfUnion::new(
                    SdfUnion::new(
                        SdfSphere::new(1.0),
                        SdfSphere::new(2.0),
                    ),
                    SdfSphere::new(3.0),
                ),
            );
        let levels_blow = evaluate_levels_below(tree.clone());
        assert_eq!(levels_blow, 3);
    }

    #[must_use]
    fn evaluate_levels_below(specimen: Rc<dyn Sdf>) -> usize {
        let mut last_levels_blow = 0;
        depth_first_search(specimen.clone(), &mut DummySdf::default(), |_, _, levels_below| {
            last_levels_blow = levels_below;
        });
        last_levels_blow
    }
}