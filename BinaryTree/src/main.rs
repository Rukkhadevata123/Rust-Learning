use std::cmp::Ord;

enum BinaryTree<T> {
    Empty,
    NonEmpty(Box<TreeNode<T>>),
}

struct TreeNode<T> {
    element: T,
    left: BinaryTree<T>,
    right: BinaryTree<T>,
}

use self::BinaryTree::*;

impl<T: Ord> BinaryTree<T> {
    fn add(&mut self, value: T) {
        match *self {
            Empty => {
                *self = NonEmpty(Box::new(TreeNode {
                    element: value,
                    left: Empty,
                    right: Empty,
                }))
            }
            NonEmpty(ref mut node) => {
                if value <= node.element {
                    node.left.add(value);
                } else {
                    node.right.add(value);
                }
            }
        }
    }
}

struct TreeIter<'a, T: 'a> {
    unvisited: Vec<&'a TreeNode<T>>,
}

impl<'a, T: 'a> TreeIter<'a, T> {
    fn push_left_edge(&mut self, mut tree: &'a BinaryTree<T>) {
        while let NonEmpty(ref node) = *tree {
            self.unvisited.push(node);
            tree = &node.left;
        }
    }
}

impl<T> BinaryTree<T> {
    fn iter(&self) -> TreeIter<T> {
        let mut iter = TreeIter {
            unvisited: Vec::new(),
        };
        iter.push_left_edge(self);
        iter
    }
}

impl<'a, T: 'a> IntoIterator for &'a BinaryTree<T> {
    type Item = &'a T;
    type IntoIter = TreeIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: 'a> Iterator for TreeIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        let node = match self.unvisited.pop() {
            None => return None,
            Some(n) => n,
        };
        self.push_left_edge(&node.right);
        Some(&node.element)
    }
}

fn main() {
    let mut tree = Empty;
    tree.add(String::from("Mercury"));
    tree.add(String::from("Venus"));
    tree.add(String::from("Earth"));
    tree.add(String::from("Mars"));
    tree.add(String::from("Jupiter"));

    let mut v = Vec::new();
    for planet in &tree {
        v.push(planet.clone());
    }
    println!("{:?}", v);
    let greetings = tree
        .iter()
        .map(|planet| format!("Hello, {}", planet))
        .collect::<Vec<_>>();
    println!("{:?}", greetings);
}
