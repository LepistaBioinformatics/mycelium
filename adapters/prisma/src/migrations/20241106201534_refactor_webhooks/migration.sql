/*
  Warnings:

  - You are about to drop the column `target` on the `webhook` table. All the data in the column will be lost.
  - A unique constraint covering the columns `[name,url,trigger]` on the table `webhook` will be added. If there are existing duplicate values, this will fail.
  - Added the required column `trigger` to the `webhook` table without a default value. This is not possible if the table is not empty.

*/
-- DropIndex
DROP INDEX "webhook_name_url_target_key";

-- AlterTable
ALTER TABLE "webhook" DROP COLUMN "target",
ADD COLUMN     "secret" JSONB,
ADD COLUMN     "trigger" VARCHAR(255) NOT NULL,
ALTER COLUMN "url" SET DATA TYPE TEXT;

-- CreateIndex
CREATE UNIQUE INDEX "webhook_name_url_trigger_key" ON "webhook"("name", "url", "trigger");
