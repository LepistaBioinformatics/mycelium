-- CreateTable
CREATE TABLE "account_tag" (
    "id" TEXT NOT NULL,
    "value" VARCHAR(64) NOT NULL,
    "meta" JSONB,
    "account_id" TEXT NOT NULL,

    CONSTRAINT "account_tag_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "account_tag_value_account_id_key" ON "account_tag"("value", "account_id");

-- AddForeignKey
ALTER TABLE "account_tag" ADD CONSTRAINT "account_tag_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE CASCADE ON UPDATE CASCADE;
