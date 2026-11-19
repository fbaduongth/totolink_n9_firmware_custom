/**
 * Binary U-Boot uImage & MTD Partition Boundary Verifier
 * Target: TOTOLINK N9v1 (CSB334A) - 8MB SPI Flash (mtd4 Firmware max = 0x6b0000 = 7,012,352 bytes)
 */
const fs = require('fs');
const path = require('path');
const zlib = require('zlib');
const crypto = require('crypto');

const UIMAGE_MAGIC = 0x27051956;
const EXPECTED_LOAD = 0x80000000;
const MTD4_MAX_BYTES = 0x006b0000; // 7,012,352 bytes (6,848 KB)

function cleanAscii(sub) {
  const nulIdx = sub.indexOf(0);
  return (nulIdx >= 0 ? sub.subarray(0, nulIdx) : sub).toString('ascii');
}

function verifyUImage(filePath) {
  const buf = fs.readFileSync(filePath);
  if (buf.length < 64) {
    throw new Error(`File too small (${buf.length} bytes)`);
  }

  const magic = buf.readUInt32BE(0);
  const hcrc = buf.readUInt32BE(4);
  const time = buf.readUInt32BE(8);
  const dataSize = buf.readUInt32BE(12);
  const loadAddr = buf.readUInt32BE(16);
  const entryAddr = buf.readUInt32BE(20);
  const dcrc = buf.readUInt32BE(24);
  const osCode = buf.readUInt8(28);
  const archCode = buf.readUInt8(29);
  const imageType = buf.readUInt8(30);
  const compType = buf.readUInt8(31);

  // Stock uImage stores product string at [32..59]; Padavan TAIL stores [ver(4B) + productid(12B)] at [32..47] and ih_ksz BE at [60..63]
  const stdName = cleanAscii(buf.subarray(32, 60));
  const padavanPid = cleanAscii(buf.subarray(36, 48));
  const isPadavanTail = padavanPid.startsWith('CSB334A-');
  const productId = isPadavanTail ? padavanPid : stdName;
  const padavanVer = isPadavanTail ? `${buf[32]}.${buf[33]}.${buf[34]}.${buf[35]}` : null;
  const ihKszBE = buf.readUInt32BE(60);

  // Compute header CRC32 with bytes [4..7] zeroed
  const hdrCopy = Buffer.from(buf.subarray(0, 64));
  hdrCopy.writeUInt32BE(0, 4);
  const calcHcrc = zlib.crc32(hdrCopy) >>> 0;

  // Compute payload CRC32 over [64 .. 64 + dataSize]
  const payload = buf.subarray(64, 64 + dataSize);
  const calcDcrc = zlib.crc32(payload) >>> 0;
  const sha256 = crypto.createHash('sha256').update(buf).digest('hex');

  const checks = {
    file: path.basename(filePath),
    totalBytes: buf.length,
    totalMiB: (buf.length / (1024 * 1024)).toFixed(2) + ' MiB',
    mtd4MaxBytes: MTD4_MAX_BYTES,
    mtd4FreeHeadroomKB: Math.floor((MTD4_MAX_BYTES - buf.length) / 1024),
    fitsMtd4: buf.length <= MTD4_MAX_BYTES && buf.length === 64 + dataSize,
    magicHex: '0x' + magic.toString(16),
    magicValid: magic === UIMAGE_MAGIC,
    headerCrcHex: '0x' + hcrc.toString(16),
    headerCrcValid: hcrc === calcHcrc,
    dataCrcHex: '0x' + dcrc.toString(16),
    dataCrcValid: dcrc === calcDcrc,
    loadAddrHex: '0x' + loadAddr.toString(16),
    loadAddrValid: loadAddr === EXPECTED_LOAD,
    entryAddrHex: '0x' + entryAddr.toString(16),
    osLinux: osCode === 5,
    archMips: archCode === 5,
    typeKernel: imageType === 2,
    compLzma: compType === 3,
    headerFormat: isPadavanTail ? 'Padavan-TAIL+uImage' : 'Standard-uImage',
    productId,
    padavanVer,
    productPrefixValid: productId.startsWith('CSB334A-'),
    ihKszHexBE: '0x' + ihKszBE.toString(16),
    sha256,
    buildTimestampUtc: new Date(time * 1000).toISOString(),
  };

  checks.allPassed = (
    checks.fitsMtd4 &&
    checks.magicValid &&
    checks.headerCrcValid &&
    checks.dataCrcValid &&
    checks.loadAddrValid &&
    checks.osLinux &&
    checks.archMips &&
    checks.typeKernel &&
    checks.productPrefixValid
  );

  return checks;
}

if (require.main === module) {
  const targets = process.argv.slice(2);
  const files = targets.length > 0
    ? targets
    : [
        path.join(__dirname, 'ConCard_Padavan_N9v1_CSB334A-2700_v1.0.web'),
        path.join(__dirname, 'ConCard_Padavan_N9v1_CSB334A-2700_v1.0.trx'),
        path.join(__dirname, 'TOTOLINK_N9v1_Stock_V4.0c.2634_Rollback.web'),
      ].filter((p) => fs.existsSync(p));

  const results = files.map(verifyUImage);
  console.log(JSON.stringify(results, null, 2));
  if (results.some((r) => !r.allPassed)) {
    process.exit(2);
  }
}

module.exports = { verifyUImage };
