-- Your SQL goes here
ALTER TABLE "coding_activities" ADD COLUMN "hidden" BOOL;
UPDATE "coding_activities" SET "hidden" = false WHERE hidden IS NULL;
ALTER TABLE "coding_activities" ALTER COLUMN hidden SET NOT NULL;







