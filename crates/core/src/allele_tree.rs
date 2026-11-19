use crate::genetics::Allele;

/// Where the World stores an Allele; Genotypes hold these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Slot(u32);

impl Slot {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }

    /// The slot with index `i`.
    pub(crate) fn at(i: usize) -> Self {
        Self(u32::try_from(i).expect("fewer than 2^32 Alleles"))
    }
}

/// Identity of an Allele within a World.
///
/// An id never comes to mean a different Allele: once its Allele is pruned,
/// queries for it find nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlleleId {
    slot: u32,
    version: u32,
}

/// Where an Allele stands in the current population.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlleleStatus {
    /// Shown by at least one live Cell.
    Living,
    /// Carried by live Cells but shown by none.
    Hidden,
    /// Carried by no live Cell; retained as an ancestor or a Founding Allele.
    Extinct,
}

/// One Allele in the Allele Tree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlleleNode {
    pub id: AlleleId,
    pub allele: Allele,
    /// The Allele this one mutated from; `None` for a Founding Allele.
    pub source: Option<AlleleId>,
    /// The Generation in which this Allele first appeared in the World.
    pub origin: u64,
    pub status: AlleleStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Record {
    allele: Allele,
    source: Option<Slot>,
    origin: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Entry {
    version: u32,
    record: Option<Record>,
}

/// Every retained Allele and which Allele each mutated from.
///
/// Founding Alleles are always retained, so the empty-World fallback of
/// Allele Pool draws can still find them. Any other Allele is pruned once no
/// live Cell carries it or any of its descendants; a later mutant may take
/// its slot under a new [`AlleleId`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AlleleTree {
    entries: Vec<Entry>,
    free: Vec<Slot>,
    founding_count: usize,
}

impl AlleleTree {
    /// A tree of Founding Alleles; the `i`th is stored in slot `i`.
    pub(crate) fn founded(alleles: &[Allele]) -> Self {
        Self {
            entries: alleles
                .iter()
                .map(|&allele| Entry {
                    version: 0,
                    record: Some(Record {
                        allele,
                        source: None,
                        origin: 0,
                    }),
                })
                .collect(),
            free: vec![],
            founding_count: alleles.len(),
        }
    }

    /// Number of Founding Alleles; they occupy slots `0..founding_count()`.
    pub(crate) fn founding_count(&self) -> usize {
        self.founding_count
    }

    /// One more than the highest slot index in use.
    pub(crate) fn slot_bound(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn allele(&self, slot: Slot) -> Allele {
        self.record(slot).allele
    }

    /// The identity of the Allele now stored in `slot`.
    pub(crate) fn id(&self, slot: Slot) -> AlleleId {
        AlleleId {
            slot: slot.0,
            version: self.entries[slot.index()].version,
        }
    }

    fn record(&self, slot: Slot) -> Record {
        self.entries[slot.index()]
            .record
            .expect("genotypes only refer to retained Alleles")
    }

    fn slot(&self, id: AlleleId) -> Option<Slot> {
        let entry = self.entries.get(id.slot as usize)?;
        (entry.version == id.version && entry.record.is_some()).then_some(Slot(id.slot))
    }

    /// Records a new Allele that mutated from `source` in Generation `origin`.
    pub(crate) fn add_mutant(&mut self, source: Slot, allele: Allele, origin: u64) -> Slot {
        let record = Some(Record {
            allele,
            source: Some(source),
            origin,
        });
        match self.free.pop() {
            Some(slot) => {
                let entry = &mut self.entries[slot.index()];
                entry.version = entry.version.wrapping_add(1);
                entry.record = record;
                slot
            }
            None => {
                let slot = Slot::at(self.entries.len());
                self.entries.push(Entry { version: 0, record });
                slot
            }
        }
    }

    /// Prunes every non-Founding Allele that is neither in `carried` nor an
    /// ancestor of an Allele in it.
    pub(crate) fn prune(&mut self, carried: impl IntoIterator<Item = Slot>) {
        let mut keep = vec![false; self.entries.len()];
        for slot in carried {
            let mut next = Some(slot);
            while let Some(slot) = next.filter(|s| !keep[s.index()]) {
                keep[slot.index()] = true;
                next = self.record(slot).source;
            }
        }
        for (index, entry) in self.entries.iter_mut().enumerate() {
            if entry.record.is_some_and(|r| r.source.is_some()) && !keep[index] {
                entry.record = None;
                self.free.push(Slot::at(index));
            }
        }
    }

    /// Every retained Allele, each before its descendants.
    pub(crate) fn nodes(&self, status: impl Fn(Slot) -> AlleleStatus) -> Vec<AlleleNode> {
        let mut nodes: Vec<AlleleNode> = (0..self.entries.len())
            .map(Slot::at)
            .filter(|slot| self.entries[slot.index()].record.is_some())
            .map(|slot| self.node(slot, &status))
            .collect();
        nodes.sort_by_key(|node| (node.origin, node.id));
        nodes
    }

    /// `id` and its ancestors back to its Founding Allele, or `None` if `id`
    /// is not retained.
    pub(crate) fn lineage(
        &self,
        id: AlleleId,
        status: impl Fn(Slot) -> AlleleStatus,
    ) -> Option<Vec<AlleleNode>> {
        let mut next = Some(self.slot(id)?);
        let mut chain = vec![];
        while let Some(slot) = next {
            chain.push(self.node(slot, &status));
            next = self.record(slot).source;
        }
        Some(chain)
    }

    fn node(&self, slot: Slot, status: impl Fn(Slot) -> AlleleStatus) -> AlleleNode {
        let record = self.record(slot);
        AlleleNode {
            id: self.id(slot),
            allele: record.allele,
            source: record.source.map(|source| self.id(source)),
            origin: record.origin,
            status: status(slot),
        }
    }
}
