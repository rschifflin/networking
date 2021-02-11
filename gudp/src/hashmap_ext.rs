use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::hash::Hash;

pub trait HashMapExt<K: Copy+Eq+Hash, V> {
  fn keys<'a>(&'a self) -> Keys<'a, K, V>;
  fn keys_ext<'a>(&self, key_buf: &'a mut Vec<K>) -> std::iter::Copied<std::slice::Iter<'a, K>>
    where K: Copy+Eq+Hash {
    key_buf.clear();
    key_buf.extend(self.keys().copied());
    key_buf.iter().copied()
  }
}

impl<K: Copy+Eq+Hash, V> HashMapExt<K,V> for HashMap<K,V> {
  fn keys<'a>(&'a self) -> Keys<'a, K, V> {
    self.keys()
  }
}
