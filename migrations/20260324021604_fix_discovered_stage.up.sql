-- Clear incorrect stage defaults for non-applied statuses.
UPDATE applications
SET stage = NULL
WHERE stage = 'applied'
  AND status IN ('discovered', 'bookmarked');
