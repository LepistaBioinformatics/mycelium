/*
  Warnings:

  - You are about to drop the `external_provider` table. If the table is not empty, all the data it contains will be lost.
  - You are about to drop the `internal_provider` table. If the table is not empty, all the data it contains will be lost.

*/
-- DropForeignKey
ALTER TABLE "external_provider" DROP CONSTRAINT "external_provider_user_id_fkey";

-- DropForeignKey
ALTER TABLE "internal_provider" DROP CONSTRAINT "internal_provider_user_id_fkey";

-- DropTable
DROP TABLE "external_provider";

-- DropTable
DROP TABLE "internal_provider";

-- CreateTable
CREATE TABLE "identity_provider" (
    "name" VARCHAR(255),
    "password_hash" VARCHAR(255),
    "password_salt" VARCHAR(255),
    "user_id" TEXT NOT NULL,

    CONSTRAINT "identity_provider_pkey" PRIMARY KEY ("user_id")
);

-- CreateIndex
CREATE UNIQUE INDEX "identity_provider_user_id_password_hash_password_salt_key" ON "identity_provider"("user_id", "password_hash", "password_salt");

-- CreateIndex
CREATE UNIQUE INDEX "identity_provider_user_id_name_key" ON "identity_provider"("user_id", "name");

-- AddForeignKey
ALTER TABLE "identity_provider" ADD CONSTRAINT "identity_provider_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
