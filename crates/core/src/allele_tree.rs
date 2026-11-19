use crate::genetics::Allele;
use crate::snapshot::{invalid, Reader, SnapshotError, Writer};

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

    /// Whether `slot` holds a retained Allele.
    pub(crate) fn is_retained(&self, slot: usize) -> bool {
        self.entries
            .get(slot)
            .is_some_and(|entry| entry.record.is_some())
    }

    /// Writes the tree in the Snapshot payload layout.
    pub(crate) fn encode(&self, writer: &mut Writer) {
        writer.index(self.founding_count);
        writer.index(self.entries.len());
        for (index, entry) in self.entries.iter().enumerate() {
            writer.u32(entry.version);
            writer.u8(u8::from(entry.record.is_some()));
            if let Some(record) = entry.record {
                writer.f64(record.allele.hue);
                writer.u8(record.allele.dominance);
                if index >= self.founding_count {
                    let source = record.source.expect("every mutant has a source");
                    writer.index(source.index());
                }
                writer.u64(record.origin);
            }
        }
        writer.index(self.free.len());
        for slot in &self.free {
            writer.index(slot.index());
        }
    }

    /// Reads a tree written by [`AlleleTree::encode`], rejecting any tree the
    /// World could not have built.
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, SnapshotError> {
        let founding_count = reader.index()?;
        let slot_count = reader.index()?;
        if founding_count == 0 || founding_count > slot_count {
            return Err(invalid(format!(
                "founding allele count {founding_count} is outside 1..={slot_count}, the slot count"
            )));
        }
        reader.expect_items(slot_count as u64, 5)?;
        let mut entries = Vec::with_capacity(slot_count);
        for index in 0..slot_count {
            let version = reader.u32()?;
            let record = match reader.u8()? {
                0 if index < founding_count => {
                    return Err(invalid(format!(
                        "founding allele slot {index} is not retained"
                    )))
                }
                0 => None,
                1 => Some(Self::decode_record(
                    reader,
                    index,
                    founding_count,
                    slot_count,
                )?),
                flag => {
                    return Err(invalid(format!(
                        "allele slot {index} has retained flag {flag}, expected 0 or 1"
                    )))
                }
            };
            entries.push(Entry { version, record });
        }
        let free_count = reader.index()?;
        reader.expect_items(free_count as u64, 4)?;
        let mut listed = vec![false; slot_count];
        let mut free = Vec::with_capacity(free_count);
        for _ in 0..free_count {
            let index = reader.index()?;
            if entries
                .get(index)
                .is_none_or(|entry| entry.record.is_some())
                || listed[index]
            {
                return Err(invalid(format!(
                    "free allele slot {index} is not an empty slot or is listed twice"
                )));
            }
            listed[index] = true;
            free.push(Slot::at(index));
        }
        if entries
            .iter()
            .filter(|entry| entry.record.is_none())
            .count()
            != free_count
        {
            return Err(invalid(
                "an empty allele slot is missing from the free slots",
            ));
        }
        let tree = Self {
            entries,
            free,
            founding_count,
        };
        tree.check_sources()?;
        Ok(tree)
    }

    fn decode_record(
        reader: &mut Reader,
        index: usize,
        founding_count: usize,
        slot_count: usize,
    ) -> Result<Record, SnapshotError> {
        let allele = Allele {
            hue: reader.f64()?,
            dominance: reader.u8()?,
        };
        if !allele.is_valid() {
            return Err(invalid(format!(
                "allele slot {index} has hue {} or dominance {} out of range",
                allele.hue, allele.dominance
            )));
        }
        let source = if index < founding_count {
            None
        } else {
            let source = reader.index()?;
            if source >= slot_count {
                return Err(invalid(format!(
                    "allele slot {index} mutated from slot {source}, beyond the slot count {slot_count}"
                )));
            }
            Some(Slot::at(source))
        };
        Ok(Record {
            allele,
            source,
            origin: reader.u64()?,
        })
    }

    /// Fails unless every retained Allele's chain of sources runs through
    /// retained Alleles back to a Founding Allele.
    fn check_sources(&self) -> Result<(), SnapshotError> {
        let mut rooted = vec![false; self.entries.len()];
        rooted[..self.founding_count].fill(true);
        for start in 0..self.entries.len() {
            if rooted[start] || self.entries[start].record.is_none() {
                continue;
            }
            let mut chain = vec![];
            let mut slot = start;
            while !rooted[slot] {
                if chain.len() == self.entries.len() {
                    return Err(invalid(format!("allele slot {start} descends from itself")));
                }
                let source = self.entries[slot]
                    .record
                    .and_then(|record| record.source)
                    .ok_or_else(|| {
                        invalid(format!(
                            "allele slot {slot} is a source but is not retained"
                        ))
                    })?;
                chain.push(slot);
                slot = source.index();
            }
            for slot in chain {
                rooted[slot] = true;
            }
        }
        Ok(())
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
