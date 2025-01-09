/*
  Warnings:

  - You are about to drop the column `totp` on the `user` table. All the data in the column will be lost.

*/
-- AlterTable
ALTER TABLE "user" DROP COLUMN "totp",
ADD COLUMN     "mfa" JSONB;
