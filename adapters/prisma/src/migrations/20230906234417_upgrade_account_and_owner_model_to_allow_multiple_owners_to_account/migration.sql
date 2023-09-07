/*
  Warnings:

  - You are about to drop the column `owner_id` on the `account` table. All the data in the column will be lost.
  - A unique constraint covering the columns `[name]` on the table `account` will be added. If there are existing duplicate values, this will fail.
  - A unique constraint covering the columns `[email,account_id]` on the table `user` will be added. If there are existing duplicate values, this will fail.
  - Added the required column `account_id` to the `user` table without a default value. This is not possible if the table is not empty.

*/
-- DropForeignKey
ALTER TABLE "account" DROP CONSTRAINT "account_owner_id_fkey";

-- DropIndex
DROP INDEX "account_name_owner_id_key";

-- DropIndex
DROP INDEX "account_owner_id_key";

-- DropIndex
DROP INDEX "user_email_key";

-- AlterTable
ALTER TABLE "account" DROP COLUMN "owner_id";

-- AlterTable
ALTER TABLE "user" ADD COLUMN     "account_id" TEXT NOT NULL;

-- CreateIndex
CREATE UNIQUE INDEX "account_name_key" ON "account"("name");

-- CreateIndex
CREATE UNIQUE INDEX "user_email_account_id_key" ON "user"("email", "account_id");

-- AddForeignKey
ALTER TABLE "user" ADD CONSTRAINT "user_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
