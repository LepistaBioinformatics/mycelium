-- CreateTable
CREATE TABLE "session_token" (
    "key" TEXT NOT NULL,
    "expiration" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "session_token_pkey" PRIMARY KEY ("key")
);
