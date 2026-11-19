use crate::genetics::Allele;

/// Identity of an Allele within a World.
///
/// Unique among the Alleles the World retains; once an Allele is pruned its
/// id may be given to a later mutant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlleleId(u32);

impl AlleleId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).expect("fewer than 2^32 retained Alleles"))
    }
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
    source: Option<AlleleId>,
    origin: u64,
}

/// Every retained Allele and which Allele each mutated from.
///
/// Founding Alleles are always retained, so they stay available as roots.
/// Any other Allele is pruned once it has no Carriers and no carried
/// descendants, and its slot is reused by a later mutant.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AlleleTree {
    records: Vec<Option<Record>>,
    free: Vec<usize>,
}

impl AlleleTree {
    /// A tree of Founding Alleles; the `i`th gets the id with index `i`.
    pub(crate) fn founded(alleles: &[Allele]) -> Self {
        Self {
            records: alleles
                .iter()
                .map(|&allele| {
                    Some(Record {
                        allele,
                        source: None,
                        origin: 0,
                    })
                })
                .collect(),
            free: vec![],
        }
    }

    /// One more than the highest index any retained Allele can have.
    pub(crate) fn capacity(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn allele(&self, id: AlleleId) -> Allele {
        self.record(id).allele
    }

    fn record(&self, id: AlleleId) -> Record {
        self.records[id.index()].expect("genotypes only refer to retained Alleles")
    }

    /// Records a new Allele that mutated from `source` in Generation `origin`.
    pub(crate) fn add_mutant(&mut self, source: AlleleId, allele: Allele, origin: u64) -> AlleleId {
        let record = Some(Record {
            allele,
            source: Some(source),
            origin,
        });
        match self.free.pop() {
            Some(index) => {
                self.records[index] = record;
                AlleleId::from_index(index)
            }
            None => {
                self.records.push(record);
                AlleleId::from_index(self.records.len() - 1)
            }
        }
    }

    /// Prunes every non-Founding Allele that is neither carried nor an
    /// ancestor of a carried Allele. `carried[i]` says whether the Allele
    /// with index `i` has a live Carrier.
    pub(crate) fn prune(&mut self, carried: &[bool]) {
        let mut keep = vec![false; self.records.len()];
        for index in (0..self.records.len()).filter(|&i| carried[i]) {
            let mut next = Some(index);
            while let Some(i) = next.filter(|&i| !keep[i]) {
                keep[i] = true;
                next = self.records[i].and_then(|r| r.source).map(AlleleId::index);
            }
        }
        for (index, slot) in self.records.iter_mut().enumerate() {
            if slot.is_some_and(|r| r.source.is_some()) && !keep[index] {
                *slot = None;
                self.free.push(index);
            }
        }
    }

    /// Every retained Allele, each before its descendants, with `status`
    /// giving the status of the Allele with index `i`.
    pub(crate) fn nodes(&self, status: impl Fn(usize) -> AlleleStatus) -> Vec<AlleleNode> {
        let mut nodes: Vec<AlleleNode> = (0..self.records.len())
            .filter_map(|i| self.node(AlleleId::from_index(i), &status))
            .collect();
        nodes.sort_by_key(|node| (node.origin, node.id));
        nodes
    }

    /// `id` and its ancestors back to its Founding Allele, or `None` if `id`
    /// is not retained.
    pub(crate) fn lineage(
        &self,
        id: AlleleId,
        status: impl Fn(usize) -> AlleleStatus,
    ) -> Option<Vec<AlleleNode>> {
        let mut chain = vec![self.node(id, &status)?];
        while let Some(source) = chain[chain.len() - 1].source {
            chain.push(self.node(source, &status)?);
        }
        Some(chain)
    }

    fn node(&self, id: AlleleId, status: impl Fn(usize) -> AlleleStatus) -> Option<AlleleNode> {
        let record = (*self.records.get(id.index())?)?;
        Some(AlleleNode {
            id,
            allele: record.allele,
            source: record.source,
            origin: record.origin,
            status: status(id.index()),
        })
    }
}
