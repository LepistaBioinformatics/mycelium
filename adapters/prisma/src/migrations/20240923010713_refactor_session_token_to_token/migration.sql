/*
  Warnings:

  - You are about to drop the `session_token` table. If the table is not empty, all the data it contains will be lost.

*/
-- DropTable
DROP TABLE "session_token";

-- CreateTable
CREATE TABLE "token" (
    "id" SERIAL NOT NULL,
    "meta" JSONB NOT NULL,
    "expiration" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "token_pkey" PRIMARY KEY ("id")
);
