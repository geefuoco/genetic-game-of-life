# Pedigrees are bounded and excluded from Snapshots

Pedigrees are kept only for Recorded History, a rolling window under a memory budget (200 MB default and maximum, lower on constrained devices), and are not part of a Snapshot. Even at about 5 bytes per Birth, a busy 1024x1024 World produces gigabytes of ancestry per hour, so unbounded history is impossible; Pedigrees are observation history rather than simulation state, so a Snapshot still resumes a run exactly. The Allele Tree, by contrast, is genetic state: it is kept in full (with extinct branches pruned) and is included in Snapshots.
