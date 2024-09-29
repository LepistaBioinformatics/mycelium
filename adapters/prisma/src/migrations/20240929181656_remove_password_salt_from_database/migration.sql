/*
  Warnings:

  - You are about to drop the column `password_salt` on the `identity_provider` table. All the data in the column will be lost.
  - A unique constraint covering the columns `[user_id,password_hash]` on the table `identity_provider` will be added. If there are existing duplicate values, this will fail.

*/
-- DropIndex
DROP INDEX "identity_provider_user_id_password_hash_password_salt_key";

-- AlterTable
ALTER TABLE "identity_provider" DROP COLUMN "password_salt";

-- CreateIndex
CREATE UNIQUE INDEX "identity_provider_user_id_password_hash_key" ON "identity_provider"("user_id", "password_hash");
