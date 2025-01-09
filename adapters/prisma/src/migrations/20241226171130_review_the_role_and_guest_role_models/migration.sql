/*
  Warnings:

  - A unique constraint covering the columns `[slug]` on the table `guest_role` will be added. If there are existing duplicate values, this will fail.
  - A unique constraint covering the columns `[name]` on the table `role` will be added. If there are existing duplicate values, this will fail.
  - A unique constraint covering the columns `[slug]` on the table `role` will be added. If there are existing duplicate values, this will fail.
  - Added the required column `slug` to the `guest_role` table without a default value. This is not possible if the table is not empty.
  - Added the required column `slug` to the `role` table without a default value. This is not possible if the table is not empty.

*/
-- AlterTable
ALTER TABLE "guest_role" ADD COLUMN     "slug" VARCHAR(140) NOT NULL;

-- AlterTable
ALTER TABLE "role" ADD COLUMN     "slug" VARCHAR(140) NOT NULL;

-- CreateIndex
CREATE UNIQUE INDEX "guest_role_slug_key" ON "guest_role"("slug");

-- CreateIndex
CREATE UNIQUE INDEX "role_name_key" ON "role"("name");

-- CreateIndex
CREATE UNIQUE INDEX "role_slug_key" ON "role"("slug");
