use std::collections::HashMap;
use std::hash::Hash;

pub fn keys_iter<'a, K, V>(map: &HashMap<K,V>, key_buf: &'a mut Vec<K>) -> impl Iterator<Item=K> + 'a
  where K: Copy+Eq+Hash, {
  key_buf.clear();
  key_buf.extend(map.keys().copied());
  key_buf.iter().copied()
}
