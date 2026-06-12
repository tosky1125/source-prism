ALTER TABLE job_attempts
    DROP CONSTRAINT job_attempts_job_id_fkey,
    ADD CONSTRAINT job_attempts_job_id_fkey
        FOREIGN KEY (job_id)
        REFERENCES jobs (job_id)
        ON DELETE CASCADE;

ALTER TABLE jobs
    DROP CONSTRAINT jobs_generation_id_fkey,
    ADD CONSTRAINT jobs_generation_id_fkey
        FOREIGN KEY (generation_id)
        REFERENCES index_generations (generation_id)
        ON DELETE CASCADE;
