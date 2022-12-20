/*
  Warnings:

  - A unique constraint covering the columns `[name]` on the table `guest_role` will be added. If there are existing duplicate values, this will fail.

*/
-- CreateIndex
CREATE UNIQUE INDEX "guest_role_name_key" ON "guest_role"("name");
