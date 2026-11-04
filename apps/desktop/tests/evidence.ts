// Screenshots and samples produced by the suite go to test-results/ by
// default; INODUNE_EVIDENCE=1 regenerates the versioned docs/evidence files.
export const EVIDENCE =
  process.env.INODUNE_EVIDENCE === '1'
    ? '../../docs/evidence'
    : 'test-results/evidence';
