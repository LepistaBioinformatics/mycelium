/*
  Warnings:

  - You are about to drop the column `permissions` on the `guest_role` table. All the data in the column will be lost.

*/
-- AlterTable
ALTER TABLE "guest_role" DROP COLUMN "permissions",
ADD COLUMN     "permission" INTEGER NOT NULL DEFAULT 0;
