/*
  Warnings:

  - You are about to drop the column `role_id` on the `guest_role` table. All the data in the column will be lost.
  - You are about to drop the `role` table. If the table is not empty, all the data it contains will be lost.
  - A unique constraint covering the columns `[name,permission]` on the table `guest_role` will be added. If there are existing duplicate values, this will fail.
  - A unique constraint covering the columns `[slug,permission]` on the table `guest_role` will be added. If there are existing duplicate values, this will fail.

*/
-- DropForeignKey
ALTER TABLE "guest_role" DROP CONSTRAINT "guest_role_role_id_fkey";

-- DropIndex
DROP INDEX "guest_role_name_key";

-- DropIndex
DROP INDEX "guest_role_slug_key";

-- AlterTable
ALTER TABLE "guest_role" DROP COLUMN "role_id";

-- DropTable
DROP TABLE "role";

-- CreateIndex
CREATE UNIQUE INDEX "guest_role_name_permission_key" ON "guest_role"("name", "permission");

-- CreateIndex
CREATE UNIQUE INDEX "guest_role_slug_permission_key" ON "guest_role"("slug", "permission");
