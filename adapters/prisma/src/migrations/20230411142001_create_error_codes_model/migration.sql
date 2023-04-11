-- CreateTable
CREATE TABLE "error_code" (
    "code" SERIAL NOT NULL,
    "prefix" TEXT NOT NULL,
    "message" VARCHAR(255) NOT NULL,
    "details" VARCHAR(255),
    "is_internal" BOOLEAN NOT NULL DEFAULT false,

    CONSTRAINT "error_code_pkey" PRIMARY KEY ("prefix","code")
);
