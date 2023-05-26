import { Footer, Text, Box, ActionIcon, FileInput, Input } from '@mantine/core';
import { useContext, useState } from 'react';
import { emit, listen } from '@tauri-apps/api/event';
import image from "../src/images/add.png"

import { open } from '@tauri-apps/api/dialog';
import { MyContext } from './Context';
import { invoke } from '@tauri-apps/api/tauri';



export default function FileAdd() {
  const { setTransfer } = useContext(MyContext);

  const openFileDialog = async () => {
    const selectedFile = await open({
      multiple: false,
    });
    console.log(selectedFile);

    const response = await emit(
      'file_upload',
      { file_path: selectedFile });
      const x = await invoke("send_active_files");
      setTransfer(x);
    

  }

  return (
    <Footer
      height={60}
    >
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-around' }} >
        <Text> Total peers connected: 20</Text>
        <ActionIcon size={48}>

          <Input
            type="button"
            icon={<img src={image} alt="My Image" style={{ width: '60px', height: '60px', marginLeft: 10 }} />}
            onClick={openFileDialog}

          />
        </ActionIcon>

        <Text> Avg Speed: 10 MB/s | 5 MB/s</Text>
      </Box>
    </Footer>
  );
}