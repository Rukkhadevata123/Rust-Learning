mod safe;
use safe::SafeList;

fn main_1() {
    let mut list = SafeList::new();

    // 添加元素
    list.push_front(3);
    list.push_front(2);
    list.push_front(1);

    println!("链表: {}", list);
    println!("链表长度: {}", list.len());

    // 迭代打印
    println!("链表元素:");
    for item in &list {
        println!("  {}", item);
    }

    // 修改元素
    for item in &mut list {
        *item *= 10;
    }

    println!("修改后: {}", list);

    // 弹出元素
    println!("弹出: {:?}", list.pop_front());
    println!("弹出后: {}", list);

    // 从数组创建
    let list_from_vec = SafeList::from(vec![5, 6, 7]);
    println!("从数组创建: {}", list_from_vec);
}

mod unsafe_list;
use unsafe_list::UnsafeList;

fn main_2() {
    // 使用 unsafe 版本的链表
    let mut list = UnsafeList::from(vec![1, 2, 3]);
    println!("原始链表: {}", list);

    list.reverse();
    println!("反转后: {}", list);

    list.push_back(4);
    println!("添加元素: {}", list);

    println!("移除索引 1 的元素: {:?}", list.remove(1));
    println!("移除后: {}", list);
}

fn main() {
    main_1();
    main_2();
}
