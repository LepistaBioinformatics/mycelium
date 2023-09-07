/*
  Warnings:

  - A unique constraint covering the columns `[name,url,target]` on the table `webhook` will be added. If there are existing duplicate values, this will fail.

*/
-- DropIndex
DROP INDEX "webhook_url_target_key";

-- AlterTable
ALTER TABLE "webhook" ALTER COLUMN "description" DROP NOT NULL;

-- CreateIndex
CREATE UNIQUE INDEX "webhook_name_url_target_key" ON "webhook"("name", "url", "target");
