mod node;

use self::node::Node;

pub struct RawART<K, V> {
    root: Node<K, V>,
    size: u64,
}

impl<K, V> RawART<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            root: Node::Nil,
            size: 0,
        }
    }

    #[inline]
    fn insert_rec(root: &mut Node<K, V>, depth: usize, key: K, value: V) {
        *root = match std::mem::replace(root, Node::Nil) {
            Node::Nil => {
                // Root empty, create initial leaf
                Node::Leaf(key, value)
            },
            Node::Node4(ptr) => {
                Node::Leaf(key, value)
            },
            Node::Node16(ptr) => {
                Node::Leaf(key, value)
            },
            Node::Node48(ptr) => {
                Node::Leaf(key, value)
            },
            Node::Node256(ptr) => {
                Node::Leaf(key, value)
            },
            Node::Leaf(k, v) => {
                Node::Leaf(key, value)
            },
        };
    }

    #[inline]
    pub fn insert(&mut self, key: &[u8], value: V, depth: usize, max_key_len: usize) {
        // insert_rec
        self.size += 1;
    }
}
