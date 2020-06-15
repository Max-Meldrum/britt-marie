use std::mem::{self, MaybeUninit};

const NODE_TYPE_4: u8 = 0;
const NODE_TYPE_16: u8 = 1;
const NODE_TYPE_48: u8 = 2;
const NODE_TYPE_256: u8 = 3;

const MAX_PREFIX_LENGTH: usize = 9;

pub enum Node<V> {
    Nil,
    Node4(Box<Node4<V>>),
    Node16(Box<Node16<V>>),
    Leaf(V),
}

pub struct NodeInfo {
    prefix_length: u32,
    count: u16,
    prefix: [u8; MAX_PREFIX_LENGTH],
}

impl NodeInfo {
    pub fn new() -> NodeInfo {
        NodeInfo {
            prefix_length: 0,
            count: 0,
            prefix: [0; MAX_PREFIX_LENGTH],
        }
    }
}

pub struct Node4<V> {
    info: NodeInfo,
    keys: [u8; 4],
    children: [Node<V>; 4],
}

impl<V> Node4<V> {
    pub fn new() -> Self {
        Self {
            info: NodeInfo::new(),
            keys: [0; 4],
            children: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }
}

pub struct Node16<V> {
    info: NodeInfo,
    keys: [u8; 16],
    children: [Node<V>; 16],
}

impl<V> Node16<V> {
    pub fn new() -> Self {
        Self {
            info: NodeInfo::new(),
            keys: [0; 16],
            children: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }
}

pub struct Node48<V> {
    info: NodeInfo,
    child_index: [u8; 256],
    children: [Node<V>; 48],
}

impl<V> Node48<V> {
    pub fn new() -> Self {
        Self {
            info: NodeInfo::new(),
            child_index: [48; 256], // Double check this..
            children: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }
}

pub struct Node256<V> {
    info: NodeInfo,
    children: [Node<V>; 256],
}

impl<V> Node256<V> {
    pub fn new() -> Self {
        Self {
            info: NodeInfo::new(),
            children: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }
}

pub trait NodeOps<V> {
    fn add_child(&mut self, node: Node<V>, byte: u8);
}

impl<V> NodeOps<V> for Node4<V> {
    fn add_child(&mut self, node: Node<V>, byte: u8) {
        let id = self.info.count;
    }
}

/*
pub fn find_child(node: *mut Node, key: u8) {
    let node_type = unsafe { (*node).node_type };
    union Nodes {
        p1: *const Node4,
        p2: *const Node16,
        p3: *const Node48,
        p4: *const Node256,
    }

    match node_type {
        NODE_TYPE_4 => {
            // linear search on the 4 nodes..
        }
        NODE_TYPE_16 => {
            // SIMD or binary search
        }
        NODE_TYPE_48 => {
            // Accessed directly through key byte
        }
        NODE_TYPE_256 => {
            // Accessed directly through key byte
            //let p = Nodes { p4:
        }
        _ => {}
    }
}
*/
