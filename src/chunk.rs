pub struct LpChunkIter<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub index: usize,
}

impl<'a> LpChunkIter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            index: 0,
        }
    }
}

impl<'a> Iterator for LpChunkIter<'a> {
    type Item = Result<(usize, &'a [u8]), &'static str>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos + 4 > self.data.len() {
            if self.pos == self.data.len() {
                return None;
            }
            return Some(Err("Incomplete length prefix"));
        }

        let chunk_len =
            u32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap()) as usize;
        self.pos += 4;

        if self.pos + chunk_len > self.data.len() {
            return Some(Err("Incomplete chunk data"));
        }

        let chunk = &self.data[self.pos..self.pos + chunk_len];
        self.pos += chunk_len;
        let current_index = self.index;
        self.index += 1;
        Some(Ok((current_index, chunk)))
    }
}
