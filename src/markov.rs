use rand::{self, prelude::*};
use std::{
  cmp::Ordering,
  collections::{BTreeMap, Bound, HashMap},
  hash::Hash,
};

type FreqTable<T> = HashMap<T, HashMap<T, f64>>;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct TotalDouble(f64);

impl Eq for TotalDouble {}

impl Ord for TotalDouble {
  fn cmp(&self, other: &Self) -> Ordering {
    self.partial_cmp(other).unwrap() // If one of the values is NaN, that's a problem anyway
  }
}

pub struct Markov<T>
where
  T: Eq,
  T: Hash,
{
  table: HashMap<T, BTreeMap<TotalDouble, T>>,
}

impl<T> Markov<T>
where
  T: Eq,
  T: Hash,
{
  pub fn new(freq: FreqTable<T>) -> Self {
    let mut table = HashMap::new();

    for (from, tos) in freq {
      let mut fold = 0.0;

      let mut new_tos = BTreeMap::new();

      for (to, freq) in tos {
        new_tos.insert(TotalDouble(fold), to);
        fold = fold + freq
      }

      table.insert(
        from,
        new_tos
          .into_iter()
          .map(|(k, v)| (TotalDouble(k.0 as f64 / fold), v))
          .collect(),
      );
    }

    Self { table }
  }

  pub fn iter<'a>(&'a self, seed: &'a T) -> MarkovIter<'a, T>
  where
    T: 'a,
  {
    MarkovIter {
      chain: self,
      state: Some(seed),
    }
  }

  pub fn iter_counted<'a>(
    &'a self,
    seed: &'a T,
    mut remain: HashMap<T, usize>,
  ) -> MarkovIterCounted<'a, T>
  where
    T: 'a,
  {
    match remain.get_mut(seed) {
      Some(n) => if *n > 0 {
        *n = *n - 1;
      } else {
        panic!()
      },
      None => panic!(),
    }

    let nremain = remain.values().fold(0, |s, n| s + n);

    MarkovIterCounted {
      chain: self,
      state: Some(seed),
      remain,
      nremain,
    }
  }

  pub fn rand_seed<'a>(&'a self) -> MarkovRandSeed<'a, T> {
    MarkovRandSeed {
      keys: self.table.keys().collect(),
    }
  }
}

pub struct MarkovIter<'a, T>
where
  T: Eq,
  T: Hash,
  T: 'a,
{
  chain: &'a Markov<T>,
  state: Option<&'a T>,
  // TODO: keep an Rng handy
}

impl<'a, T> Iterator for MarkovIter<'a, T>
where
  T: Eq,
  T: Hash,
{
  type Item = &'a T;

  fn next(&mut self) -> Option<Self::Item> {
    let state = self.state;

    if let Some(state) = state {
      self.state = self.chain.table.get(state).map(|map| {
        let f = TotalDouble(rand::thread_rng().gen_range(0.0, 1.0));

        map
          .range((Bound::Unbounded, Bound::Included(f)))
          .next_back()
          .unwrap()
          .1
      });
    }

    state
  }
}

pub struct MarkovIterCounted<'a, T>
where
  T: Eq,
  T: Hash,
  T: 'a,
{
  chain: &'a Markov<T>,
  state: Option<&'a T>,
  remain: HashMap<T, usize>,
  nremain: usize,
  // TODO: keep an Rng handy
}

impl<'a, T> Iterator for MarkovIterCounted<'a, T>
where
  T: Eq,
  T: Hash,
  T: 'a,
{
  type Item = &'a T;

  fn next(&mut self) -> Option<Self::Item> {
    let state = self.state;

    if let Some(state) = state {
      // TODO: this approach is really dumb and susceptible to hanging; it should
      //       probably be fixed by constructing a temporary state table
      self.state = if self.nremain > 0 {
        self.chain.table.get(state).map(|map| loop {
          let f = TotalDouble(rand::thread_rng().gen_range(0.0, 1.0));

          let (_, val) = map
            .range((Bound::Unbounded, Bound::Included(f)))
            .next_back()
            .unwrap();

          if let Some(n) = self.remain.get_mut(val) {
            if *n > 0 {
              *n = *n - 1;
              self.nremain = self.nremain - 1;
              break val;
            }
          }
        })
      } else {
        None
      };
    }

    state
  }
}

pub struct MarkovRandSeed<'a, T>
where
  T: Eq,
  T: Hash,
  T: 'a,
{
  keys: Vec<&'a T>,
  // TODO: keep an Rng handy
}

impl<'a, T> Iterator for MarkovRandSeed<'a, T>
where
  T: Eq,
  T: Hash,
  T: 'a,
{
  type Item = &'a T;

  fn next(&mut self) -> Option<Self::Item> {
    let i = rand::thread_rng().gen_range(0, self.keys.len());
    Some(self.keys[i])
  }
}

pub fn analyze_corpus<I, J, T>(i: I) -> FreqTable<T>
where
  I: IntoIterator<Item = (f64, J)>,
  J: IntoIterator<Item = T>,
  T: Eq,
  T: Hash,
  T: Clone,
{
  let mut table = FreqTable::new();

  for (weight, j) in i {
    let mut prev = None;

    for el in j {
      use std::collections::hash_map::Entry::*;

      if let Some(prev) = prev {
        match match table.entry(prev) {
          Vacant(v) => v.insert(HashMap::new()),
          Occupied(o) => o.into_mut(),
        }.entry(el.clone())
        {
          Vacant(v) => {
            v.insert(weight);
          }
          Occupied(o) => {
            let o = o.into_mut();
            *o = *o + weight;
          }
        }
      }

      prev = Some(el);
    }
  }

  table
}
