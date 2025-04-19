#! /usr/bin/env python3

import logging
import os
import subprocess
from datetime import datetime, timedelta

import click
import requests

# ------------------------------------------------------------------------------
# Setup logging
# ------------------------------------------------------------------------------


LOG_DIR = "/var/log/mycelium/healthcheck_partition_manager"
os.makedirs(LOG_DIR, exist_ok=True)
log_file = os.path.join(LOG_DIR, "partition_manager.log")


logging.basicConfig(
    filename=log_file,
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(message)s",
)


# ------------------------------------------------------------------------------
# Send alert to Telegram
# ------------------------------------------------------------------------------


def __send_telegram_alert(token: str, chat_id: str, message: str) -> None:
    """Send an alert to Telegram.

    Args:
        token (str): Telegram bot token.
        chat_id (str): Telegram chat ID.
        message (str): Message to send.

    Raises:
        Exception: If the request to Telegram fails.
    """

    try:
        url = f"https://api.telegram.org/bot{token}/sendMessage"
        data = {"chat_id": chat_id, "text": message}
        requests.post(url, data=data, timeout=10)

    except Exception as e:
        logging.error(f"Failed to send alert to Telegram: {e}")


# ------------------------------------------------------------------------------
# Main
# ------------------------------------------------------------------------------


@click.command()
@click.option(
    "--db-name",
    required=True,
    help="Database name",
)
@click.option(
    "--db-user",
    required=True,
    help="Database user",
)
@click.option(
    "--db-host",
    default="localhost",
    help="Database host",
)
@click.option(
    "--azure-container",
    help="Azure Blob container",
)
@click.option(
    "--azure-account",
    help="Azure storage account",
)
@click.option(
    "--gcs-bucket",
    help="Google Cloud Storage bucket",
)
@click.option(
    "--telegram-token",
    required=True,
    help="Telegram bot token",
)
@click.option(
    "--telegram-chat-id",
    required=True,
    help="Telegram chat ID",
)
@click.option(
    "--backup-dir",
    default="/tmp/healthcheck_backups",
    help="Temporary backup directory",
)
def __main(
    db_name: str,
    db_user: str,
    db_host: str,
    azure_container: str,
    azure_account: str,
    gcs_bucket: str,
    telegram_token: str,
    telegram_chat_id: str,
    backup_dir: str,
) -> None:
    """Main function.

    Args:
        db_name (str): Database name.
        db_user (str): Database user.
        db_host (str): Database host.
        azure_container (str): Azure container.
        azure_account (str): Azure storage account.
        gcs_bucket (str): Google Cloud Storage bucket.
        telegram_token (str): Telegram bot token.
        telegram_chat_id (str): Telegram chat ID.
        backup_dir (str): Temporary backup directory.

    Raises:
        click.UsageError: If no storage service is selected.
        Exception: If an error occurs.
    """

    #
    # Validate that at least one storage service was selected
    #
    if not any([all([azure_container, azure_account]), gcs_bucket]):
        raise click.UsageError(
            "You must specify at least one storage service: "
            "Azure (--azure-container and --azure-account) or "
            "Google Cloud Storage (--gcs-bucket)"
        )

    os.makedirs(backup_dir, exist_ok=True)

    today = datetime.now().date()
    tomorrow = today + timedelta(days=1)
    day_after = today + timedelta(days=2)
    drop_date = today - timedelta(days=15)

    partition_today = f"healthcheck_logs_{today.strftime('%Y%m%d')}"
    partition_tomorrow = f"healthcheck_logs_{tomorrow.strftime('%Y%m%d')}"
    partition_drop = f"healthcheck_logs_{drop_date.strftime('%Y%m%d')}"
    tsv_file = f"{partition_drop}.tsv"
    tsv_path = os.path.join(backup_dir, tsv_file)

    try:
        #
        # Create partitions
        #
        logging.info("Creating current and next day partitions...")

        partition_sql = f"""
        DO $$
        BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM pg_tables WHERE tablename = '{partition_today}'
            ) THEN
                EXECUTE 'CREATE TABLE {partition_today}
                         PARTITION OF healthcheck_logs
                         FOR VALUES FROM (''{today}'') TO (''{tomorrow}'')';
            END IF;

            IF NOT EXISTS (
                SELECT 1 FROM pg_tables WHERE tablename = '{partition_tomorrow}'
            ) THEN
                EXECUTE 'CREATE TABLE {partition_tomorrow}
                         PARTITION OF healthcheck_logs
                         FOR VALUES FROM (''{tomorrow}'') TO (''{day_after}'')';
            END IF;
        END $$;
        """

        create_partitions = subprocess.run(
            [
                "psql",
                "-U",
                db_user,
                "-d",
                db_name,
                "-h",
                db_host,
                "-c",
                partition_sql,
            ],
            check=True,
            capture_output=True,
        )

        if create_partitions.returncode != 0:
            logging.error(f"Failed to create partitions: {create_partitions.stderr}")
            __send_telegram_alert(telegram_token, telegram_chat_id, f"🚨 {msg}")

            return

        #
        # Export TSV
        #
        logging.info(f"Exporting TSV from partition {partition_drop}...")
        export_tsv = subprocess.run(
            [
                "psql",
                "-U",
                db_user,
                "-d",
                db_name,
                "-h",
                db_host,
                "-c",
                f"COPY {partition_drop} TO '{tsv_path}' WITH DELIMITER E'\t';",
            ],
            check=True,
            capture_output=True,
        )

        if export_tsv.returncode != 0:
            logging.error(f"Failed to export TSV: {export_tsv.stderr}")
            __send_telegram_alert(telegram_token, telegram_chat_id, f"🚨 {msg}")

            return

        #
        # Upload Azure
        #
        if azure_container and azure_account:
            logging.info("Sending TSV to Azure Blob Storage...")
            upload_azure = subprocess.run(
                [
                    "az",
                    "storage",
                    "blob",
                    "upload",
                    "--account-name",
                    azure_account,
                    "--container-name",
                    azure_container,
                    "--name",
                    f"backups/{tsv_file}",
                    "--file",
                    tsv_path,
                ],
                check=True,
                capture_output=True,
            )

            if upload_azure.returncode != 0:
                logging.error(
                    f"Falha ao fazer upload do TSV para Azure: {upload_azure.stderr}"
                )
                __send_telegram_alert(telegram_token, telegram_chat_id, f"🚨 {msg}")

                return

        #
        # Upload GCS
        #
        if gcs_bucket:
            logging.info("Sending TSV to Google Cloud Storage...")
            upload_gcs = subprocess.run(
                [
                    "gsutil",
                    "cp",
                    tsv_path,
                    f"gs://{gcs_bucket}/backups/{tsv_file}",
                ],
                check=True,
                capture_output=True,
            )

            if upload_gcs.returncode != 0:
                logging.error(f"Failed to upload TSV to GCS: {upload_gcs.stderr}")
                __send_telegram_alert(telegram_token, telegram_chat_id, f"🚨 {msg}")

                return

        #
        # Drop partition
        #
        logging.info(f"Dropping partition {partition_drop}...")

        drop_partition = subprocess.run(
            [
                "psql",
                "-U",
                db_user,
                "-d",
                db_name,
                "-h",
                db_host,
                "-c",
                f"DROP TABLE IF EXISTS {partition_drop} CASCADE;",
            ],
            check=True,
            capture_output=True,
        )

        if drop_partition.returncode != 0:
            logging.error(f"Failed to drop partition: {drop_partition.stderr}")
            __send_telegram_alert(telegram_token, telegram_chat_id, f"🚨 {msg}")

            return

        logging.info("Process completed successfully.")

    except subprocess.CalledProcessError as e:
        msg = f"Error processing partition: {e}"
        logging.error(msg)
        __send_telegram_alert(telegram_token, telegram_chat_id, f"🚨 {msg}")

    except Exception as e:
        msg = f"Unexpected error: {e}"
        logging.error(msg)
        __send_telegram_alert(telegram_token, telegram_chat_id, f"🚨 {msg}")


if __name__ == "__main__":
    __main()
