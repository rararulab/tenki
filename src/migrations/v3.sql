UPDATE applications
SET stage = NULL
WHERE stage = 'applied'
  AND status IN ('discovered', 'bookmarked');
