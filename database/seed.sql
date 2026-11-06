-- Optional local-only sample data. Run manually after migrations.
-- First register a local account with the email demo@example.test.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM users WHERE email = 'demo@example.test') THEN
        RAISE EXCEPTION 'Register a local account with email demo@example.test before applying this seed';
    END IF;
END
$$;

INSERT INTO tasks (id, user_id, title, description, completed)
SELECT sample.id, users.id, sample.title, sample.description, sample.completed
FROM users
CROSS JOIN (
    VALUES
        ('b1b8c2f0-3040-4b9e-8b2a-0b0a7e5d3111'::UUID, 'Explore Nexora', 'Review the three-tier application.', FALSE),
        ('b1b8c2f0-3040-4b9e-8b2a-0b0a7e5d3222'::UUID, 'Ship a small improvement', 'Pick one task and make steady progress.', TRUE)
) AS sample(id, title, description, completed)
WHERE users.email = 'demo@example.test'
ON CONFLICT (id) DO NOTHING;
