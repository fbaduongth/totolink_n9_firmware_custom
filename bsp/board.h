/* TOTOLINK N9v1 (CaryStudio CSB334A) - Concard N9 Hybrid Enterprise AP Edition */

#define BOARD_PID		"CSB334A-2700"
#define BOARD_NAME		"Concard-N9"
#define BOARD_DESC		"Concard N9 Hybrid Enterprise Ceiling AP"
#define BOARD_VENDOR_NAME	"Viettechsoft"
#define BOARD_VENDOR_URL	"https://viettechsoft.com/"
#define BOARD_MODEL_URL		"https://viettechsoft.com/"
#define BOARD_BOOT_TIME		20
#define BOARD_FLASH_TIME	90

/* Verified Hardware GPIO Pinout (CSB334A rev 0154) */
#define BOARD_GPIO_BTN_RESET	13	/* Active-low Reset button */
#undef  BOARD_GPIO_BTN_WPS
#undef  BOARD_GPIO_LED_ALL
#undef  BOARD_GPIO_LED_WIFI
#define BOARD_GPIO_LED_POWER	11	/* Ceiling Ring Blue/Green System LED */
#undef  BOARD_GPIO_LED_LAN
#define BOARD_GPIO_LED_WAN	39	/* Ceiling Ring Red Alert LED (kept OFF in normal AP operation) */
#define BOARD_GPIO_LED_USB	3	/* USB 2.0 LED */
#undef  BOARD_GPIO_LED_ROUTER
#undef  BOARD_GPIO_PWR_USB

/* Pure Access Point Default Role (sw_mode=3, all ESW ports 0..4 bridged in br0) */
#define BOARD_ROLE_REPEATER	1

/* Radio & Switch Capabilities: MT7620A 2T2R 300Mbps + External PA/LNA, 128MB DDR2, 8MB SPI */
#define BOARD_RAM_SIZE		128
#define BOARD_HAS_5G_11AC	0
#define BOARD_NUM_ANT_5G_TX	0
#define BOARD_NUM_ANT_5G_RX	0
#define BOARD_NUM_ANT_2G_TX	2
#define BOARD_NUM_ANT_2G_RX	2
#define BOARD_NUM_ETH_LEDS	0
#define BOARD_NUM_ETH_EPHY	5
#define BOARD_HAS_EPHY_L1000	0
#define BOARD_HAS_EPHY_W1000	0
