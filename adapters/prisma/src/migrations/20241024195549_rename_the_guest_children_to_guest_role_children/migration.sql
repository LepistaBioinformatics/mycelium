/*
  Warnings:

  - You are about to drop the `guest_children` table. If the table is not empty, all the data it contains will be lost.

*/
-- DropForeignKey
ALTER TABLE "guest_children" DROP CONSTRAINT "guest_children_child_role_id_fkey";

-- DropForeignKey
ALTER TABLE "guest_children" DROP CONSTRAINT "guest_children_parent_id_fkey";

-- DropTable
DROP TABLE "guest_children";

-- CreateTable
CREATE TABLE "guest_role_children" (
    "id" TEXT NOT NULL,
    "parent_id" TEXT NOT NULL,
    "child_role_id" TEXT NOT NULL,

    CONSTRAINT "guest_role_children_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "guest_role_children_parent_id_child_role_id_key" ON "guest_role_children"("parent_id", "child_role_id");

-- AddForeignKey
ALTER TABLE "guest_role_children" ADD CONSTRAINT "guest_role_children_parent_id_fkey" FOREIGN KEY ("parent_id") REFERENCES "guest_role"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "guest_role_children" ADD CONSTRAINT "guest_role_children_child_role_id_fkey" FOREIGN KEY ("child_role_id") REFERENCES "guest_role"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
