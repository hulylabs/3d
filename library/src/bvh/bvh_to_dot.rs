use crate::bvh::node::BvhNode;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
use crate::geometry::utils::debug_format_human_readable_point;

/*

This module provides functionality to visualize BVH trees using the Graphviz DOT format.
The generated DOT files can be rendered using Graphviz tools like `dot`, `neato`, etc.

save_bvh_as_dot_detailed(&bvh_root, "bvh_tree_detailed.dot").unwrap();

Render with Graphviz (from command line):
dot -Tpng bvh_tree.dot -o bvh_tree.png
dot -Tsvg bvh_tree_detailed.dot -o bvh_tree_detailed.svg

*/
pub(crate) fn save_bvh_as_dot_detailed<DescriptionDelegate: Fn(Option<usize>)->String>(
    root: &Rc<RefCell<BvhNode>>,
    describe: DescriptionDelegate,
    file_path: impl AsRef<Path>,
) -> Result<(), std::io::Error> {
    let mut dot_content = String::new();
    
    dot_content.push_str("digraph BVH {\n");
    dot_content.push_str("    rankdir=TB;\n");
    dot_content.push_str("    node [shape=box, style=rounded];\n\n");
    
    build_dot_content(&mut dot_content, root, describe);
    
    dot_content.push_str("}\n");
    
    let mut file = File::create(file_path)?;
    file.write_all(dot_content.as_bytes())?;
    
    Ok(())
}

fn create_node_label(node: &BvhNode, description: String) -> String {
    let mut label = String::new();

    if let Some(idx) = node.serial_index() {
        label.push_str(&format!("#{idx}"));
    }

    if let Some(content_type) = node.content_type()  {
        label.push_str(format!("\n{content_type:?}: {description}").as_str());
    }

    let human_readable_min = debug_format_human_readable_point(node.aabb().min());
    let human_readable_max = debug_format_human_readable_point(node.aabb().max());
    label.push_str(format!("\n[min({human_readable_min})\nmax({human_readable_max})]").as_str());
    
    label.push_str(format!("\nmiss->{:?}", node.miss_node_index_or_null()).as_str());
    
    label
}

fn build_dot_content<DescriptionDelegate: Fn(Option<usize>)->String>(
    content: &mut String,
    root: &Rc<RefCell<BvhNode>>,
    describe: DescriptionDelegate,
) {
    let mut counter: usize = 0;
    let mut stack = vec![(root.clone(), None, false)];
    let mut node_ids = std::collections::HashMap::new();
    
    while let Some((node, parent_node_id, is_visited)) = stack.pop() {
        if is_visited {
            let node_reference = node.borrow();
            let current_id = *node_ids.get(&node.as_ptr()).unwrap();
            
            let is_leaf = node_reference.left().is_none() && node_reference.right().is_none();
            let color = if is_leaf { "lightgreen" } else { "lightblue" };
            
            content.push_str(&format!(
                "    n{} [label=\"{}\", fillcolor={}, style=filled];\n",
                current_id,
                create_node_label(&node_reference, describe(node_reference.content_index())),
                color
            ));
            
            if let Some(parent) = parent_node_id {
                content.push_str(&format!("    n{parent} -> n{current_id};\n"));
            }
        } else {
            let current_id = counter;
            counter += 1;
            node_ids.insert(node.as_ptr(), current_id);
            
            stack.push((node.clone(), parent_node_id, true));
            
            let node_ref = node.borrow();
            if let Some(right) = node_ref.right() {
                stack.push((right.clone(), Some(current_id), false));
            }
            if let Some(left) = node_ref.left() {
                stack.push((left.clone(), Some(current_id), false));
            }
        }
    }
}
