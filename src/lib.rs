pub mod art;

use art::ART;

// TODO
pub struct BrittMarie<K, V>
where
    K: AsRef<[u8]>,
{
    art: ART<K, V>,
}
