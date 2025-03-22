use std::fmt::{self, Debug};

/// 单向链表的节点
#[derive(Debug)]
struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

/// 安全单向链表
#[derive(Debug)]
pub struct SafeList<T> {
    head: Option<Box<Node<T>>>,
    len: usize,
}

impl<T> SafeList<T> {
    /// 创建一个空链表
    pub fn new() -> Self {
        SafeList { head: None, len: 0 }
    }

    /// 判断链表是否为空
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// 获取链表长度
    pub fn len(&self) -> usize {
        self.len
    }

    /// 在链表头部添加元素
    pub fn push_front(&mut self, value: T) {
        let new_node = Box::new(Node {
            value,
            next: self.head.take(),
        });
        self.head = Some(new_node);
        self.len += 1;
    }

    /// 从链表头部移除元素
    pub fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|mut head| {
            self.head = head.next.take();
            self.len -= 1;
            head.value
        })
    }

    /// 查看链表头部元素，不移除
    pub fn peek_front(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.value)
    }

    /// 查看链表头部元素的可变引用
    pub fn peek_front_mut(&mut self) -> Option<&mut T> {
        self.head.as_mut().map(|node| &mut node.value)
    }

    /// 清空链表
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// 在链表尾部添加元素
    pub fn push_back(&mut self, value: T) {
        let new_node = Box::new(Node { value, next: None });

        let mut current = &mut self.head;

        while let Some(ref mut node) = *current {
            current = &mut node.next;
        }

        *current = Some(new_node);
        self.len += 1;
    }

    /// 将链表转换为迭代器
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            next: self.head.as_deref(),
        }
    }

    /// 将链表转换为可变迭代器
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            next: self.head.as_deref_mut(),
        }
    }
}

/// 链表的迭代器
pub struct Iter<'a, T> {
    next: Option<&'a Node<T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.map(|node| {
            self.next = node.next.as_deref();
            &node.value
        })
    }
}

/// 链表的可变迭代器
pub struct IterMut<'a, T> {
    next: Option<&'a mut Node<T>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|node| {
            self.next = node.next.as_deref_mut();
            &mut node.value
        })
    }
}

/// 实现 IntoIterator 特性，支持 for 循环遍历
impl<'a, T> IntoIterator for &'a SafeList<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SafeList<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// 消费型迭代器
pub struct IntoIter<T>(SafeList<T>);

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }
}

impl<T> IntoIterator for SafeList<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self)
    }
}

/// 默认实现
impl<T> Default for SafeList<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// 可以从数组中创建链表
impl<T> From<Vec<T>> for SafeList<T> {
    fn from(vec: Vec<T>) -> Self {
        let mut list = SafeList::new();
        for item in vec.into_iter().rev() {
            list.push_front(item);
        }
        list
    }
}

/// 实现格式化打印
impl<T: Debug> fmt::Display for SafeList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;

        let mut iter = self.iter();
        if let Some(first) = iter.next() {
            write!(f, "{:?}", first)?;
            for item in iter {
                write!(f, ", {:?}", item)?;
            }
        }

        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_pop() {
        let mut list = SafeList::new();

        assert_eq!(list.len(), 0);
        assert!(list.is_empty());

        list.push_front(1);
        list.push_front(2);
        list.push_front(3);

        assert_eq!(list.len(), 3);
        assert!(!list.is_empty());

        assert_eq!(list.pop_front(), Some(3));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), None);

        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }

    #[test]
    fn test_peek() {
        let mut list = SafeList::new();
        assert_eq!(list.peek_front(), None);

        list.push_front(1);
        list.push_front(2);

        assert_eq!(list.peek_front(), Some(&2));

        if let Some(value) = list.peek_front_mut() {
            *value = 42;
        }

        assert_eq!(list.peek_front(), Some(&42));
    }

    #[test]
    fn test_iterators() {
        let mut list = SafeList::new();
        list.push_front(1);
        list.push_front(2);
        list.push_front(3);

        // 借用迭代
        let mut iter_values = Vec::new();
        for &value in &list {
            iter_values.push(value);
        }
        assert_eq!(iter_values, vec![3, 2, 1]);

        // 可变迭代
        for value in &mut list {
            *value *= 10;
        }

        // 消费迭代
        let collected: Vec<_> = list.into_iter().collect();
        assert_eq!(collected, vec![30, 20, 10]);
    }

    #[test]
    fn test_push_back() {
        let mut list = SafeList::new();
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);

        let collected: Vec<_> = list.into_iter().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn test_from_vec() {
        let vec = vec![1, 2, 3];
        let list = SafeList::from(vec);

        let back_to_vec: Vec<_> = list.into_iter().collect();
        assert_eq!(back_to_vec, vec![1, 2, 3]);
    }
}
