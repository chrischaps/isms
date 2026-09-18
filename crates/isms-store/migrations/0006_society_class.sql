-- S1.16 (ADR-0009): a third society class, `lab`, where synthetic players may
-- play. A lab society is never listed or served on /public/*, never counted in
-- a cross-society comparison, and never canonical. The column keeps its TEXT
-- type; the check makes the comment enforceable.
ALTER TABLE societies
    ADD CONSTRAINT societies_class_check CHECK (class IN ('canonical', 'community', 'lab'));
COMMENT ON COLUMN societies.class IS 'canonical | community | lab (ADR-0009: lab is hidden from /public)';
