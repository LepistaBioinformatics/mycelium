/*
  Warnings:

  - The primary key for the `guest_role_children` table will be changed. If it partially fails, the table could be left without primary key constraint.
  - You are about to drop the column `id` on the `guest_role_children` table. All the data in the column will be lost.

*/
-- DropIndex
DROP INDEX "guest_role_children_parent_id_child_role_id_key";

-- AlterTable
ALTER TABLE "guest_role_children" DROP CONSTRAINT "guest_role_children_pkey",
DROP COLUMN "id",
ADD CONSTRAINT "guest_role_children_pkey" PRIMARY KEY ("parent_id", "child_role_id");
