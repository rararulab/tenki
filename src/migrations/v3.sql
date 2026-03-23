-- v3: clear misleading default stage for pre-application statuses

-- Historical rows imported/bookmarked under old schema may have stage='applied'
-- due to default value, even when the status is not yet applied.
UPDATE applications
SET stage = NULL
WHERE status IN ('discovered', 'bookmarked')
  AND stage = 'applied';
