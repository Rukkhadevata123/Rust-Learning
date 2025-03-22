use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;

/// 使用裸指针实现的节点
#[derive(Debug)]
struct Node<T> {
    value: T,
    next: Option<NonNull<Node<T>>>,
}

/// 使用 unsafe 实现的链表
pub struct UnsafeList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    /// 使用 PhantomData 标记 T 的所有权
    _marker: PhantomData<Box<Node<T>>>,
}

// 为了让 UnsafeList 能够安全地跨线程共享
unsafe impl<T: Send> Send for UnsafeList<T> {}
unsafe impl<T: Sync> Sync for UnsafeList<T> {}

impl<T: Debug> Debug for UnsafeList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnsafeList[")?;
        let mut current = self.head;
        let mut first = true;

        while let Some(node) = current {
            if !first {
                write!(f, ", ")?;
            } else {
                first = false;
            }

            unsafe {
                write!(f, "{:?}", (*node.as_ptr()).value)?;
                current = (*node.as_ptr()).next;
            }
        }

        write!(f, "]")
    }
}

impl<T> UnsafeList<T> {
    /// 创建一个新的空链表
    pub fn new() -> Self {
        UnsafeList {
            head: None,
            tail: None,
            len: 0,
            _marker: PhantomData,
        }
    }

    /// 检查链表是否为空
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// 返回链表长度
    pub fn len(&self) -> usize {
        self.len
    }

    /// 在链表头部插入元素
    pub fn push_front(&mut self, value: T) {
        // 创建一个堆分配的节点
        let node = Box::new(Node {
            value,
            next: self.head,
        });

        // 将 Box 转换为裸指针
        let node_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(node)) };

        if self.head.is_none() {
            // 如果链表为空，设置尾指针
            self.tail = Some(node_ptr);
        }

        // 更新头指针
        self.head = Some(node_ptr);
        self.len += 1;
    }

    /// 在链表尾部插入元素，O(1) 操作
    pub fn push_back(&mut self, value: T) {
        // 创建一个堆分配的节点
        let node = Box::new(Node { value, next: None });

        let node_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(node)) };

        // 如果链表不为空，更新尾节点的 next 指针
        if let Some(tail) = self.tail {
            unsafe {
                (*tail.as_ptr()).next = Some(node_ptr);
            }
        } else {
            // 如果链表为空，设置头指针
            self.head = Some(node_ptr);
        }

        // 更新尾指针
        self.tail = Some(node_ptr);
        self.len += 1;
    }

    /// 删除链表头部元素
    pub fn pop_front(&mut self) -> Option<T> {
        self.head.map(|head_ptr| unsafe {
            // 转换回 Box，使 Rust 接管内存管理
            let head = Box::from_raw(head_ptr.as_ptr());

            // 更新头指针
            self.head = head.next;

            // 如果头部为空，也更新尾指针
            if self.head.is_none() {
                self.tail = None;
            }

            self.len -= 1;
            head.value
        })
    }

    /// 获取头部元素的引用，不移除
    pub fn peek_front(&self) -> Option<&T> {
        unsafe { self.head.map(|head| &(*head.as_ptr()).value) }
    }

    /// 获取头部元素的可变引用
    pub fn peek_front_mut(&mut self) -> Option<&mut T> {
        unsafe { self.head.map(|head| &mut (*head.as_ptr()).value) }
    }

    /// 清空链表，释放所有节点
    pub fn clear(&mut self) {
        // 消耗整个链表
        while self.pop_front().is_some() {}
    }

    /// 反转链表
    pub fn reverse(&mut self) {
        // 使用裸指针操作，O(n) 时间、O(1) 空间
        let mut prev = None;
        let mut current = self.head;

        // 保存原尾节点，因为它将成为新的头节点
        let new_head = self.tail;

        while let Some(curr_ptr) = current {
            unsafe {
                // 保存下一个节点
                let next = (*curr_ptr.as_ptr()).next;

                // 反转指针
                (*curr_ptr.as_ptr()).next = prev;

                // 向前移动
                prev = Some(curr_ptr);
                current = next;
            }
        }

        // 更新头尾指针
        self.tail = self.head;
        self.head = new_head;
    }

    /// 获取迭代器
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            current: self.head,
            _marker: PhantomData,
        }
    }

    /// 获取可变迭代器
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            current: self.head,
            _marker: PhantomData,
        }
    }

    /// 删除指定索引位置的节点
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }

        if index == 0 {
            return self.pop_front();
        }

        let mut i = 0;
        let mut current = self.head;
        let mut prev: Option<NonNull<Node<T>>> = None;

        // 找到前一个节点
        while i < index && current.is_some() {
            unsafe {
                prev = current;
                current = current.and_then(|curr| (*curr.as_ptr()).next);
                i += 1;
            }
        }

        // 删除当前节点
        if let (Some(prev_ptr), Some(curr_ptr)) = (prev, current) {
            unsafe {
                let curr = Box::from_raw(curr_ptr.as_ptr());
                (*prev_ptr.as_ptr()).next = curr.next;

                // 如果删除的是尾节点，需要更新尾指针
                if curr.next.is_none() {
                    self.tail = prev;
                }

                self.len -= 1;
                Some(curr.value)
            }
        } else {
            None
        }
    }
}

// 迭代器实现
pub struct Iter<'a, T> {
    current: Option<NonNull<Node<T>>>,
    _marker: PhantomData<&'a Node<T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.current.map(|curr| unsafe {
            // 获取当前节点的引用
            let current = &*curr.as_ptr();
            // 移动到下一个节点
            self.current = current.next;
            // 返回值的引用
            &current.value
        })
    }
}

// 可变迭代器实现
pub struct IterMut<'a, T> {
    current: Option<NonNull<Node<T>>>,
    _marker: PhantomData<&'a mut Node<T>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.current.map(|curr| unsafe {
            // 获取当前节点的可变引用
            let current = &mut *curr.as_ptr();
            // 移动到下一个节点
            self.current = current.next;
            // 返回值的可变引用
            &mut current.value
        })
    }
}

// 消费型迭代器
impl<T> IntoIterator for UnsafeList<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        IntoIter(self)
    }
}

pub struct IntoIter<T>(UnsafeList<T>);

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }
}

// 借用迭代
impl<'a, T> IntoIterator for &'a UnsafeList<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// 可变借用迭代
impl<'a, T> IntoIterator for &'a mut UnsafeList<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

// 默认实现
impl<T> Default for UnsafeList<T> {
    fn default() -> Self {
        Self::new()
    }
}

// 从向量创建链表
impl<T> From<Vec<T>> for UnsafeList<T> {
    fn from(vec: Vec<T>) -> Self {
        let mut list = UnsafeList::new();
        for item in vec {
            list.push_back(item);
        }
        list
    }
}

// Drop 实现，确保所有节点被正确释放
impl<T> Drop for UnsafeList<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

// 格式化输出实现
impl<T: Debug> fmt::Display for UnsafeList<T> {
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

// 实现链表拼接操作
impl<T> UnsafeList<T> {
    pub fn append(&mut self, mut other: Self) {
        if other.is_empty() {
            return;
        }

        if self.is_empty() {
            // 如果当前链表为空，直接使用另一个链表
            mem::swap(self, &mut other);
            return;
        }

        // 连接两个链表
        if let Some(tail) = self.tail {
            unsafe {
                (*tail.as_ptr()).next = other.head;
            }
            self.tail = other.tail;
            self.len += other.len;
        }

        // 防止 other 在 drop 时释放节点
        other.head = None;
        other.tail = None;
        other.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let mut list = UnsafeList::new();
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
    fn test_push_back() {
        let mut list = UnsafeList::new();

        list.push_back(1);
        list.push_back(2);
        list.push_back(3);

        assert_eq!(list.len(), 3);

        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), Some(3));
    }

    #[test]
    fn test_peek() {
        let mut list = UnsafeList::new();
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
    fn test_reverse() {
        let mut list = UnsafeList::from(vec![1, 2, 3, 4, 5]);
        list.reverse();

        let items: Vec<_> = list.into_iter().collect();
        assert_eq!(items, vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_remove() {
        let mut list = UnsafeList::from(vec![1, 2, 3, 4, 5]);

        // 删除中间元素
        assert_eq!(list.remove(2), Some(3));
        assert_eq!(list.len(), 4);

        // 删除头部元素
        assert_eq!(list.remove(0), Some(1));
        assert_eq!(list.len(), 3);

        // 删除尾部元素
        assert_eq!(list.remove(2), Some(5));
        assert_eq!(list.len(), 2);

        // 超出范围
        assert_eq!(list.remove(5), None);

        let items: Vec<_> = list.into_iter().collect();
        assert_eq!(items, vec![2, 4]);
    }

    #[test]
    fn test_append() {
        let mut list1 = UnsafeList::from(vec![1, 2, 3]);
        let list2 = UnsafeList::from(vec![4, 5, 6]);

        list1.append(list2);

        let items: Vec<_> = list1.into_iter().collect();
        assert_eq!(items, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_iter_mut() {
        let mut list = UnsafeList::from(vec![1, 2, 3]);

        for item in &mut list {
            *item *= 10;
        }

        let items: Vec<_> = list.into_iter().collect();
        assert_eq!(items, vec![10, 20, 30]);
    }
}
