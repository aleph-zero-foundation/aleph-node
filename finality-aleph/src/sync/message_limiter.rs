use std::fmt::{Display, Formatter};

use parity_scale_codec::Encode;

use crate::sync::data::MAX_SYNC_MESSAGE_SIZE;

const MSG_BYTES_LIMIT: usize = MAX_SYNC_MESSAGE_SIZE as usize;

pub struct Limiter<'a, D: Encode, const LIMIT: usize> {
    msg: &'a [D],
    start_index: usize,
}

pub type MsgLimiter<'a, D> = Limiter<'a, D, MSG_BYTES_LIMIT>;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    ItemTooBig,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ItemTooBig => write!(f, "Single item takes more than the limit"),
        }
    }
}

impl<'a, D: Encode, const LIMIT: usize> Limiter<'a, D, LIMIT> {
    pub fn new(msg: &'a [D]) -> Self {
        Self {
            msg,
            start_index: 0,
        }
    }

    pub fn next_largest_msg(&mut self) -> Result<Option<&'a [D]>, Error> {
        if self.start_index == self.msg.len() {
            return Ok(None);
        }
        let end_idx = self.find_idx_of_largest_prefix()?;

        let start_index = self.start_index;
        self.start_index = end_idx;

        Ok(Some(&self.msg[start_index..end_idx]))
    }

    fn find_idx_of_largest_prefix(&self) -> Result<usize, Error> {
        let mut idx = self.start_index;
        let mut encoded_sum = 0;

        while idx < self.msg.len() && encoded_sum <= LIMIT {
            encoded_sum += self.msg[idx].encoded_size();
            idx += 1;
        }

        // encoded size of the msg[start_index..idx] may be larger than the limit. Trim last items
        // until the encoded size fits into the limit.
        while idx > self.start_index && self.msg[self.start_index..idx].encoded_size() > LIMIT {
            idx -= 1;
        }

        if idx == self.start_index {
            Err(Error::ItemTooBig)
        } else {
            Ok(idx)
        }
    }
}

#[cfg(test)]
mod tests {
    use parity_scale_codec::Encode;

    use crate::sync::message_limiter::{Error, Limiter};

    type TestLimiter<'a, D> = Limiter<'a, D, 10>;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct EncodeToSize(usize);

    impl Encode for EncodeToSize {
        fn size_hint(&self) -> usize {
            self.0
        }
        fn encode(&self) -> Vec<u8> {
            vec![0; self.0]
        }
    }

    fn sized(size: usize) -> EncodeToSize {
        EncodeToSize(size)
    }

    #[test]
    fn takes_one_that_fit() {
        let v = vec![sized(5), sized(6), sized(7)];

        let mut lim = TestLimiter::new(&v);

        assert_eq!(Ok(Some(&v[..1])), lim.next_largest_msg())
    }
    #[test]
    fn takes_all() {
        let v = vec![sized(1), sized(2), sized(3)];

        let mut lim = TestLimiter::new(&v);

        assert_eq!(Ok(Some(&v[..])), lim.next_largest_msg())
    }
    #[test]
    fn takes_all_that_fits_into_limit() {
        let v = vec![sized(1), sized(2), sized(7)];

        let mut lim = TestLimiter::new(&v);

        assert_eq!(Ok(Some(&v[..2])), lim.next_largest_msg())
    }
    #[test]
    fn works_with_empty_input() {
        let v = vec![];

        let mut lim = TestLimiter::<EncodeToSize>::new(&v);

        assert_eq!(Ok(None), lim.next_largest_msg())
    }
    #[test]
    fn respects_the_limit() {
        let v = vec![sized(10)];

        let mut lim = TestLimiter::new(&v);

        assert_eq!(Err(Error::ItemTooBig), lim.next_largest_msg())
    }

    #[test]
    fn iterates_correctly() {
        let v = vec![sized(5), sized(6), sized(7)];

        let mut lim = TestLimiter::new(&v);

        assert_eq!(Ok(Some(&v[..1])), lim.next_largest_msg());
        assert_eq!(Ok(Some(&v[1..2])), lim.next_largest_msg());
        assert_eq!(Ok(Some(&v[2..3])), lim.next_largest_msg());
        assert_eq!(Ok(None), lim.next_largest_msg());
    }

    #[test]
    fn iterates_correctly_2() {
        let v = vec![
            sized(5),
            sized(3),
            sized(2),
            sized(5),
            sized(5),
            sized(6),
            sized(7),
        ];

        let mut lim = TestLimiter::new(&v);

        assert_eq!(Ok(Some(&v[..2])), lim.next_largest_msg());
        assert_eq!(Ok(Some(&v[2..4])), lim.next_largest_msg());
        assert_eq!(Ok(Some(&v[4..5])), lim.next_largest_msg());
        assert_eq!(Ok(Some(&v[5..6])), lim.next_largest_msg());
        assert_eq!(Ok(Some(&v[6..7])), lim.next_largest_msg());
        assert_eq!(Ok(None), lim.next_largest_msg());
    }

    #[test]
    fn iterates_correctly_with_oversized_element() {
        let v = vec![
            sized(5),
            sized(3),
            sized(2),
            sized(5),
            sized(5),
            sized(6),
            sized(10), // should not be included
        ];

        let mut lim = TestLimiter::new(&v);

        assert_eq!(Ok(Some(&v[..2])), lim.next_largest_msg());
        assert_eq!(Ok(Some(&v[2..4])), lim.next_largest_msg());
        assert_eq!(Ok(Some(&v[4..5])), lim.next_largest_msg());
        assert_eq!(Ok(Some(&v[5..6])), lim.next_largest_msg());
        assert_eq!(Err(Error::ItemTooBig), lim.next_largest_msg());
    }
}
