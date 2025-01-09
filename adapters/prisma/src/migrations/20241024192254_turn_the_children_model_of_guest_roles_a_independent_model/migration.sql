/*
  Warnings:

  - You are about to drop the column `children` on the `guest_role` table. All the data in the column will be lost.

*/
-- AlterTable
ALTER TABLE "guest_role" DROP COLUMN "children";

-- CreateTable
CREATE TABLE "guest_children" (
    "id" TEXT NOT NULL,
    "parent_id" TEXT NOT NULL,
    "child_role_id" TEXT NOT NULL,

    CONSTRAINT "guest_children_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "guest_children_parent_id_child_role_id_key" ON "guest_children"("parent_id", "child_role_id");

-- AddForeignKey
ALTER TABLE "guest_children" ADD CONSTRAINT "guest_children_parent_id_fkey" FOREIGN KEY ("parent_id") REFERENCES "guest_role"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "guest_children" ADD CONSTRAINT "guest_children_child_role_id_fkey" FOREIGN KEY ("child_role_id") REFERENCES "guest_role"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
