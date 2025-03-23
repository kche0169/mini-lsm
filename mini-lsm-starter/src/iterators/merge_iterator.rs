// Copyright (c) 2022-2025 Alex Chi Z
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use anyhow::Result;
use std::cmp::{self};
use std::collections::BinaryHeap;
use std::collections::binary_heap::PeekMut;

use crate::key::KeySlice;

use super::StorageIterator;

struct HeapWrapper<I: StorageIterator>(pub usize, pub Box<I>);

impl<I: StorageIterator> PartialEq for HeapWrapper<I> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl<I: StorageIterator> Eq for HeapWrapper<I> {}

impl<I: StorageIterator> PartialOrd for HeapWrapper<I> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: StorageIterator> Ord for HeapWrapper<I> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.1
            .key()
            .cmp(&other.1.key())
            .then(self.0.cmp(&other.0))
            .reverse()
    }
}

/// Merge multiple iterators of the same type. If the same key occurs multiple times in some
/// iterators, prefer the one with smaller index.
pub struct MergeIterator<I: StorageIterator> {
    iters: BinaryHeap<HeapWrapper<I>>,
    current: Option<HeapWrapper<I>>,
}

impl<I: StorageIterator> MergeIterator<I> {
    pub fn create(iters: Vec<Box<I>>) -> Self {
        let mut iter = BinaryHeap::from(
            iters
                .into_iter()
                .filter(|x| x.is_valid())
                .enumerate()
                .map(|(i, x)| HeapWrapper(i, x))
                .collect::<Vec<HeapWrapper<I>>>(),
        );
        let current = iter.pop();
        Self {
            iters: iter,
            current: current,
        }
    }
}

impl<I: 'static + for<'a> StorageIterator<KeyType<'a> = KeySlice<'a>>> StorageIterator
    for MergeIterator<I>
{
    type KeyType<'a> = KeySlice<'a>;

    fn key(&self) -> KeySlice {
        if let Some(x) = self.current.as_ref() {
            x.1.key().clone()
        } else {
            KeySlice::default()
        }
    }

    fn value(&self) -> &[u8] {
        if let Some(x) = self.current.as_ref() {
            x.1.value()
        } else {
            &[]
        }
    }

    fn is_valid(&self) -> bool {
        // 1. current 必须存在
        // 2. 当前迭代器处于有效状态
        self.current.is_some() && self.current.as_ref().unwrap().1.is_valid()
        //         self.current.is_some() && self.current.as_ref().unwrap().1.is_valid()
    }

    fn next(&mut self) -> Result<()> {
        let c = match self.current.as_mut() {
            Some(x) => x,
            None => return Ok(()),
        };
        while let Some(mut wrapper) = self.iters.peek_mut() {
            if wrapper.1.key() == c.1.key() {
                if let e @ Err(_) = wrapper.1.next() {
                    PeekMut::pop(wrapper);
                    return e;
                }
                if !wrapper.1.is_valid() {
                    PeekMut::pop(wrapper);
                }
            } else {
                break;
            }
        }
        c.1.next()?;
        if c.1.is_valid() {
            if let Some(mut peek_wrapper) = self.iters.peek_mut() {
                if *c < *peek_wrapper {
                    std::mem::swap(&mut *peek_wrapper, c);
                }
            }
        } else {
            self.current = self.iters.pop();
        }
        Ok(())
    }
}
